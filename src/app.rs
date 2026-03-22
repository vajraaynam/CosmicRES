// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Length, Subscription, window::Id};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use sysinfo::{Disks, System, Networks};

// ────────────────────────────────────────────────────────────────────────────
// Resource statistics
// ────────────────────────────────────────────────────────────────────────────

/// A snapshot of current CPU / RAM / disk usage.
#[derive(Debug, Default, Clone)]
pub struct ResourceStats {
    pub cpu_usage: f32,
    pub used_memory: u64,
    pub total_memory: u64,
    pub used_disk: u64,
    pub total_disk: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

impl ResourceStats {
    pub fn ram_percent(&self) -> f32 {
        if self.total_memory == 0 {
            return 0.0;
        }
        (self.used_memory as f32 / self.total_memory as f32) * 100.0
    }

    pub fn disk_percent(&self) -> f32 {
        if self.total_disk == 0 {
            return 0.0;
        }
        (self.used_disk as f32 / self.total_disk as f32) * 100.0
    }

    pub fn format_bytes(bytes: u64) -> String {
        const GIB: u64 = 1024 * 1024 * 1024;
        const MIB: u64 = 1024 * 1024;
        const KIB: u64 = 1024;
        if bytes >= GIB {
            format!("{:.1} GiB", bytes as f64 / GIB as f64)
        } else if bytes >= MIB {
            format!("{:.1} MiB", bytes as f64 / MIB as f64)
        } else if bytes >= KIB {
            format!("{:.1} KiB", bytes as f64 / KIB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

fn collect_stats(sys: &mut System, networks: &mut Networks) -> ResourceStats {
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu_usage = sys.global_cpu_usage();
    let used_memory = sys.used_memory();
    let total_memory = sys.total_memory();

    let disks = Disks::new_with_refreshed_list();
    let (used_disk, total_disk) = disks.iter().fold((0u64, 0u64), |(used, total), d| {
        (
            used + (d.total_space() - d.available_space()),
            total + d.total_space(),
        )
    });

    networks.refresh(true);
    let (rx_bytes, tx_bytes) = networks.iter().fold((0u64, 0u64), |(rx, tx), (_, d)| {
        (rx + d.received(), tx + d.transmitted())
    });

    ResourceStats { cpu_usage, used_memory, total_memory, used_disk, total_disk, rx_bytes, tx_bytes }
}

// ────────────────────────────────────────────────────────────────────────────
// Background tick stream (must be a fn pointer for Subscription::run_with)
// ────────────────────────────────────────────────────────────────────────────

fn tick_stream(_id: &std::any::TypeId) -> impl cosmic::iced::futures::Stream<Item = Message> + use<> {
    use futures::SinkExt;
    cosmic::iced::stream::channel(
        4,
        |mut channel: futures::channel::mpsc::Sender<Message>| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let _ = channel.send(Message::Tick).await;
            }
        },
    )
}

// ────────────────────────────────────────────────────────────────────────────
// App model
// ────────────────────────────────────────────────────────────────────────────

pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    stats: ResourceStats,
    sys: System,
    networks: Networks,
}

impl Default for AppModel {
    fn default() -> Self {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_all();
        networks.refresh(true);
        let stats = collect_stats(&mut sys, &mut networks);
        Self { core: cosmic::Core::default(), popup: None, config: Config::default(), stats, sys, networks }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    Tick,
    UpdateConfig(Config),
    ToggleShowDisk(bool),
}

// ────────────────────────────────────────────────────────────────────────────
// cosmic::Application impl
// ────────────────────────────────────────────────────────────────────────────

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.github.pop-os.cosmic-resource-monitor";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_all();
        networks.refresh(true);
        let stats = collect_stats(&mut sys, &mut networks);

        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let app = AppModel { core, popup: None, config, stats, sys, networks };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Panel button: shows multiple metrics with icons.
    fn view(&self) -> Element<'_, Self::Message> {
        let stats = &self.stats;
        
        // Helper to create a small metric group: Icon + Text
        let metric = |icon_name: &str, value: String| {
            widget::row()
                .spacing(4)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::icon::from_name(icon_name).size(14))
                .push(widget::text(value).size(14))
        };

        let content = widget::row()
            .spacing(12)
            .align_y(cosmic::iced::Alignment::Center)
            .push(metric("cpu-symbolic", format!("{:.0}%", stats.cpu_usage)))
            .push(metric("memory-symbolic", format!("{:.0}%", stats.ram_percent())))
            .push(metric("drive-harddisk-symbolic", format!("{:.0}%", stats.disk_percent())))
            .push(metric("network-transmit-receive-symbolic", ResourceStats::format_bytes(stats.rx_bytes + stats.tx_bytes)));

        self.core
            .applet
            .button_from_element(content, false)
            .on_press(Message::TogglePopup)
            .into()
    }

    /// Popup window with CPU / RAM / Disk progress bars.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let stats = &self.stats;

        let cpu_row = widget::column()
            .push(
                widget::row()
                    .push(widget::text(fl!("cpu-usage")).width(Length::Fill))
                    .push(widget::text(format!("{:.1}%", stats.cpu_usage))),
            )
            .push(widget::progress_bar(0.0..=100.0, stats.cpu_usage))
            .spacing(4);

        let ram_row = widget::column()
            .push(
                widget::row()
                    .push(widget::text(fl!("ram-usage")).width(Length::Fill))
                    .push(widget::text(format!(
                        "{} / {}",
                        ResourceStats::format_bytes(stats.used_memory),
                        ResourceStats::format_bytes(stats.total_memory)
                    ))),
            )
            .push(widget::progress_bar(0.0..=100.0, stats.ram_percent()))
            .spacing(4);

        let disk_row = widget::column()
            .push(
                widget::row()
                    .push(widget::text(fl!("disk-usage")).width(Length::Fill))
                    .push(widget::text(format!(
                        "{} / {}",
                        ResourceStats::format_bytes(stats.used_disk),
                        ResourceStats::format_bytes(stats.total_disk)
                    ))),
            )
            .push(widget::progress_bar(0.0..=100.0, stats.disk_percent()))
            .spacing(4);

        let toggle_row = widget::settings::item(
            fl!("show-disk"),
            widget::toggler(self.config.show_disk).on_toggle(Message::ToggleShowDisk),
        );

        let net_rx_row = widget::row()
            .push(widget::text(fl!("network-rx")).width(Length::Fill))
            .push(widget::text(ResourceStats::format_bytes(stats.rx_bytes)));

        let net_tx_row = widget::row()
            .push(widget::text(fl!("network-tx")).width(Length::Fill))
            .push(widget::text(ResourceStats::format_bytes(stats.tx_bytes)));

        let mut content = widget::list_column()
            .padding(12)
            .spacing(12)
            .add(cpu_row)
            .add(ram_row);

        if self.config.show_disk {
            content = content.add(disk_row);
        }

        content = content
            .add(widget::divider::horizontal::default())
            .add(net_rx_row)
            .add(net_tx_row)
            .add(widget::divider::horizontal::default())
            .add(toggle_row);

        self.core.applet.popup_container(content).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        struct TickSub;

        Subscription::batch(vec![
            Subscription::run_with(std::any::TypeId::of::<TickSub>(), tick_stream),
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Tick => {
                self.stats = collect_stats(&mut self.sys, &mut self.networks);
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleShowDisk(show) => {
                self.config.show_disk = show;
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
