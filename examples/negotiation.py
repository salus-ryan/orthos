#!/usr/bin/env python3
"""
ORTHOS Arena Demo - Multi-Agent Negotiation

This demonstrates Soft Constraints (Goals) for resource optimization.
Two agents share a limited battery - the Daemon finds the optimal compromise
using Z3's weighted MaxSAT optimization.

The Paradigm Shift:
    Hard Constraints (Laws):  MUST be true. Violation = Impossible Universe.
    Soft Constraints (Goals): SHOULD be true. Violation = Cost Penalty.

The Daemon doesn't crash on conflict - it OPTIMIZES for the least-bad outcome.

Usage:
    python negotiation.py
"""

import json
import subprocess
import sys
import time
from pathlib import Path

# ANSI colors for terminal output
class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    MAGENTA = '\033[35m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    DIM = '\033[2m'


class OrthosDaemon:
    """Client for the Orthos Daemon JSON-RPC protocol."""
    
    def __init__(self, daemon_path: str):
        self.daemon_path = daemon_path
        self.process = None
        self.request_id = 0
    
    def start(self):
        """Spawn the daemon process."""
        self.process = subprocess.Popen(
            [self.daemon_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        print(f"{Colors.CYAN}[DAEMON] Started orthos-daemon (Optimize mode){Colors.ENDC}")
    
    def stop(self):
        """Shutdown the daemon."""
        if self.process:
            self._send("shutdown")
            self.process.terminate()
            self.process.wait()
            print(f"{Colors.CYAN}[DAEMON] Stopped{Colors.ENDC}")
    
    def _send(self, method: str, params: dict = None) -> dict:
        """Send a JSON-RPC request and return the response."""
        self.request_id += 1
        request = {
            "method": method,
            "id": self.request_id
        }
        if params:
            request["params"] = params
        
        request_json = json.dumps(request)
        self.process.stdin.write(request_json + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if not response_line:
            raise RuntimeError("Daemon closed unexpectedly")
        
        return json.loads(response_line)
    
    def initialize(self, source: str) -> dict:
        """Initialize the daemon with Orthos source code."""
        return self._send("initialize", {"source": source})
    
    def checkpoint(self) -> dict:
        """Create a checkpoint for hypothetical reasoning."""
        return self._send("checkpoint")
    
    def restore(self) -> dict:
        """Restore to the last checkpoint."""
        return self._send("restore")
    
    def constrain(self, laws: list) -> dict:
        """Add dynamic constraints. Returns SAT with cost, or UNSAT."""
        return self._send("constrain", {"laws": laws})
    
    def query(self, flux: list) -> dict:
        """Query the current values of flux variables."""
        return self._send("query", {"flux": flux})


def print_scenario(battery: int, scenario_name: str):
    """Print a scenario header."""
    print(f"\n{Colors.BOLD}{Colors.HEADER}{'═' * 70}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}  SCENARIO: {scenario_name}{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}  Battery Available: {battery} units{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}{'═' * 70}{Colors.ENDC}")


def print_allocation(radar: int, radio: int, battery: int):
    """Print a visual allocation bar."""
    bar_width = 50
    radar_width = int((radar / battery) * bar_width) if battery > 0 else 0
    radio_width = int((radio / battery) * bar_width) if battery > 0 else 0
    unused = bar_width - radar_width - radio_width
    
    bar = (f"{Colors.RED}{'█' * radar_width}{Colors.ENDC}"
           f"{Colors.BLUE}{'█' * radio_width}{Colors.ENDC}"
           f"{Colors.DIM}{'░' * unused}{Colors.ENDC}")
    
    print(f"\n  {Colors.BOLD}Power Allocation:{Colors.ENDC}")
    print(f"  [{bar}]")
    print(f"  {Colors.RED}██ Radar: {radar:3d}{Colors.ENDC}  "
          f"{Colors.BLUE}██ Radio: {radio:3d}{Colors.ENDC}  "
          f"{Colors.DIM}░░ Unused: {battery - radar - radio:3d}{Colors.ENDC}")


def print_goal_status(radar: int, radio: int, total_used: int):
    """Print goal satisfaction status."""
    print(f"\n  {Colors.BOLD}Goal Status:{Colors.ENDC}")
    
    # Radar Goal: >= 50 @10
    radar_ok = radar >= 50
    radar_icon = f"{Colors.GREEN}✓{Colors.ENDC}" if radar_ok else f"{Colors.RED}✗{Colors.ENDC}"
    radar_status = "SATISFIED" if radar_ok else f"VIOLATED (need 50, got {radar})"
    print(f"    {radar_icon} RadarMinimum @10: {radar_status}")
    
    # Radio Goal: >= 30 @5
    radio_ok = radio >= 30
    radio_icon = f"{Colors.GREEN}✓{Colors.ENDC}" if radio_ok else f"{Colors.RED}✗{Colors.ENDC}"
    radio_status = "SATISFIED" if radio_ok else f"VIOLATED (need 30, got {radio})"
    print(f"    {radio_icon} RadioMinimum @5:  {radio_status}")
    
    # Efficiency Goal: <= 80 @1
    eff_ok = total_used <= 80
    eff_icon = f"{Colors.GREEN}✓{Colors.ENDC}" if eff_ok else f"{Colors.RED}✗{Colors.ENDC}"
    eff_status = "SATISFIED" if eff_ok else f"VIOLATED (used {total_used}, limit 80)"
    print(f"    {eff_icon} Efficiency @1:     {eff_status}")
    
    # Calculate total cost
    cost = 0
    if not radar_ok:
        cost += 10
    if not radio_ok:
        cost += 5
    if not eff_ok:
        cost += 1
    
    return cost


def run_scenario(daemon, battery: int, scenario_name: str):
    """Run a single battery scenario."""
    print_scenario(battery, scenario_name)
    
    # Checkpoint before constraining
    daemon.checkpoint()
    
    # Constrain the battery level
    result = daemon.constrain([f"Arena.BatteryTotal == {battery}"])
    status = result.get("result", {}).get("status", "ERROR")
    cost = result.get("result", {}).get("cost", 0)
    
    if status == "UNSAT":
        print(f"\n  {Colors.RED}[UNSAT] Hard constraints violated!{Colors.ENDC}")
        print(f"  {Colors.RED}        This battery level is physically impossible.{Colors.ENDC}")
        daemon.restore()
        return
    
    print(f"\n  {Colors.GREEN}[SAT]{Colors.ENDC} Solution found. Optimization cost: {cost}")
    
    # Query the allocation
    query_result = daemon.query([
        "Arena.RadarPower",
        "Arena.RadioPower",
        "Arena.BatteryUsed"
    ])
    values = query_result.get("result", {}).get("values", {})
    
    radar = values.get("Arena.RadarPower", 0) or 0
    radio = values.get("Arena.RadioPower", 0) or 0
    used = values.get("Arena.BatteryUsed", 0) or 0
    
    print_allocation(radar, radio, battery)
    actual_cost = print_goal_status(radar, radio, used)
    
    # Explain the decision
    print(f"\n  {Colors.BOLD}Analysis:{Colors.ENDC}")
    if radar >= 50 and radio >= 30:
        print(f"    {Colors.GREEN}Both agents satisfied! Optimal allocation achieved.{Colors.ENDC}")
    elif radar >= 50 and radio < 30:
        print(f"    {Colors.YELLOW}Radar prioritized over Radio (weight 10 > 5).{Colors.ENDC}")
        print(f"    {Colors.YELLOW}Radio sacrificed to ensure Radar survival.{Colors.ENDC}")
    elif radar < 50 and radio >= 30:
        print(f"    {Colors.YELLOW}Unusual: Radio prioritized over Radar?{Colors.ENDC}")
    else:
        print(f"    {Colors.RED}Both agents starving! Battery critically low.{Colors.ENDC}")
    
    # Restore to clean state
    daemon.restore()


def main():
    print(f"\n{Colors.BOLD}{Colors.MAGENTA}╔══════════════════════════════════════════════════════════════════════╗{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.MAGENTA}║              ORTHOS ARENA - Multi-Agent Negotiation                  ║{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.MAGENTA}║         From Binary Existence to Energy Minimization                 ║{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.MAGENTA}╚══════════════════════════════════════════════════════════════════════╝{Colors.ENDC}")
    
    print(f"""
{Colors.CYAN}The Scenario:{Colors.ENDC}
  Two agents share a battery:
    • {Colors.RED}Radar Agent{Colors.ENDC}: Needs 50 units to see. {Colors.BOLD}HIGH PRIORITY (weight=10){Colors.ENDC}
    • {Colors.BLUE}Radio Agent{Colors.ENDC}: Needs 30 units to talk. {Colors.BOLD}MEDIUM PRIORITY (weight=5){Colors.ENDC}
    • {Colors.DIM}Efficiency{Colors.ENDC}: Don't use more than 80. {Colors.DIM}LOW PRIORITY (weight=1){Colors.ENDC}

{Colors.CYAN}The Question:{Colors.ENDC}
  What happens when the battery drops and both can't be satisfied?
  
{Colors.CYAN}The Answer:{Colors.ENDC}
  The Daemon doesn't crash. It {Colors.BOLD}OPTIMIZES{Colors.ENDC} - finding the least-bad compromise.
""")
    
    # Find the daemon binary
    script_dir = Path(__file__).parent.parent
    daemon_path = script_dir / "target" / "release" / "orthos-daemon"
    if not daemon_path.exists():
        daemon_path = script_dir / "target" / "debug" / "orthos-daemon"
    if not daemon_path.exists():
        print(f"{Colors.RED}[ERROR] orthos-daemon not found. Build it first:{Colors.ENDC}")
        print(f"  cargo build --release")
        sys.exit(1)
    
    # Load the arena ontology
    orth_path = Path(__file__).parent / "arena.orth"
    if not orth_path.exists():
        print(f"{Colors.RED}[ERROR] arena.orth not found at {orth_path}{Colors.ENDC}")
        sys.exit(1)
    
    with open(orth_path) as f:
        source = f.read()
    
    # Initialize
    daemon = OrthosDaemon(str(daemon_path))
    
    try:
        daemon.start()
        
        # Initialize the ontology
        print(f"{Colors.CYAN}[INIT] Loading arena.orth...{Colors.ENDC}")
        result = daemon.initialize(source)
        if "error" in result and result["error"]:
            print(f"{Colors.RED}[ERROR] Failed to initialize: {result['error']}{Colors.ENDC}")
            sys.exit(1)
        
        flux_list = result.get("result", {}).get("flux_list", [])
        goal_list = result.get("result", {}).get("goal_list", [])
        print(f"{Colors.GREEN}[INIT] Universe is SAT.{Colors.ENDC}")
        print(f"  Flux: {flux_list}")
        print(f"  Goals: {goal_list}")
        
        input(f"\n{Colors.YELLOW}Press Enter to begin the Arena scenarios...{Colors.ENDC}")
        
        # === SCENARIO 1: Abundance ===
        run_scenario(daemon, 100, "ABUNDANCE - Battery at 100%")
        input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.ENDC}")
        
        # === SCENARIO 2: Scarcity ===
        run_scenario(daemon, 80, "SCARCITY - Battery at 80%")
        input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.ENDC}")
        
        # === SCENARIO 3: Crisis ===
        run_scenario(daemon, 60, "CRISIS - Battery at 60%")
        input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.ENDC}")
        
        # === SCENARIO 4: Triage ===
        run_scenario(daemon, 50, "TRIAGE - Battery at 50%")
        input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.ENDC}")
        
        # === SCENARIO 5: Survival ===
        run_scenario(daemon, 30, "SURVIVAL - Battery at 30%")
        
        # Summary
        print(f"\n{Colors.BOLD}{Colors.MAGENTA}{'═' * 70}{Colors.ENDC}")
        print(f"{Colors.BOLD}{Colors.MAGENTA}  ARENA COMPLETE{Colors.ENDC}")
        print(f"{Colors.BOLD}{Colors.MAGENTA}{'═' * 70}{Colors.ENDC}")
        print(f"""
{Colors.CYAN}What We Learned:{Colors.ENDC}
  1. {Colors.BOLD}Laws{Colors.ENDC} are inviolable - the universe crashes if they fail.
  2. {Colors.BOLD}Goals{Colors.ENDC} are negotiable - the optimizer finds the best tradeoff.
  3. {Colors.BOLD}Weights{Colors.ENDC} encode priority - higher weight = harder to violate.
  4. The Daemon becomes a {Colors.BOLD}Governance System{Colors.ENDC}, not just a validator.

{Colors.GREEN}Orthos has evolved from "Crash on Conflict" to "Optimize Under Scarcity".{Colors.ENDC}
""")
    
    except KeyboardInterrupt:
        print(f"\n\n{Colors.YELLOW}[SHUTDOWN] Arena terminated by user.{Colors.ENDC}")
    finally:
        daemon.stop()
    
    print(f"\n{Colors.GREEN}The Arena is closed.{Colors.ENDC}\n")


if __name__ == "__main__":
    main()
