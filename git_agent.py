import subprocess
import time
import sys
import argparse
from datetime import datetime

# Gen Z Vibes Commit Messages
MESSAGES = [
    "updates are straight fire 🔥",
    "vibes are high, code is slay 💅",
    "no cap, just pushing code 🧢",
    "bet, more changes inbound 📈",
    "it's giving... progress ☕",
    "main character energy in this commit 👑",
    "we strictly out here grinding 💯",
    "caught the vibe, updated the repo ✨"
]

def run_git_command(args, dry_run=False):
    if dry_run:
        print(f"[DRY RUN] Would run: git {' '.join(args)}")
        return True
    
    try:
        result = subprocess.run(["git"] + args, check=True, capture_output=True, text=True)
        return True
    except subprocess.CalledProcessError as e:
        print(f"L (Error): {e.stderr.strip()}", file=sys.stderr)
        return False

def check_remote():
    try:
        result = subprocess.run(["git", "remote"], capture_output=True, text=True)
        return bool(result.stdout.strip())
    except:
        return False

def sync(interval=1800, dry_run=False):
    print("🚀 Git Management Agent is vibing... (Press Ctrl+C to stop)")
    
    while True:
        if not check_remote() and not dry_run:
            print("⚠️ Yo, no remote found! Add one with: git remote add origin <url>")
            print("Skipping push for now, but still committing local changes. Bet.")
        
        # Check for changes
        status = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True).stdout.strip()
        
        if status:
            print(f"[{datetime.now().strftime('%H:%M:%S')}] Changes detected! Slaying... 💅")
            
            # git add . (Honors .gitignore, no cap)
            run_git_command(["add", "."], dry_run)
            
            # git commit -m
            msg = MESSAGES[int(time.time()) % len(MESSAGES)]
            run_git_command(["commit", "-m", msg], dry_run)
            
            # git push origin main
            if check_remote() or dry_run:
                run_git_command(["push", "origin", "main"], dry_run)
            
            print("✅ Pushed successfully. No cap. 🧢")
        else:
            print(f"[{datetime.now().strftime('%H:%M:%S')}] No changes. Just vibing. 🧊")
        
        if dry_run:
            print("[DRY RUN] Finished one cycle. Stopping.")
            break
            
        time.sleep(interval)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Git Management Agent (Gen Z Edition)")
    parser.add_argument("--interval", type=int, default=1800, help="Interval in seconds (default: 30 mins)")
    parser.add_argument("--dry-run", action="store_true", help="Run once without actually pushing")
    
    args = parser.parse_args()
    
    try:
        sync(args.interval, args.dry_run)
    except KeyboardInterrupt:
        print("\nPeace out! ✌️")
