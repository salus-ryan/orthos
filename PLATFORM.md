# ORTHOS Platform

Orthos is now a **Platform** with two evolutionary paths implemented.

## Path A: Standard Library (Evolution)

Users no longer need to define physics and economics from scratch.

### `std/physics.orth`
- **Velocity** / **Velocity2D** - Kinematics
- **Mass** - Newton's Second Law
- **Momentum** - Conservation laws
- **Collision1D** / **InelasticCollision** - Collision mechanics
- **Gravity** - Free fall
- **Energy** - Kinetic & potential
- **Spring** - Hooke's Law
- **Friction** - Static/kinetic
- **Pressure** / **Density** / **Wave** - Fluid & wave mechanics

### `std/economics.orth`
- **Supply** / **Demand** - Market curves
- **MarketEquilibrium** - Price discovery
- **Transaction** - Buyer/seller exchange
- **Budget** / **Profit** / **Production** - Business constraints
- **Inventory** / **Utility** - Resource management
- **ExchangeRate** / **Loan** / **Tax** - Financial instruments
- **Auction** / **Scarcity** - Allocation problems

## Path B: Embodiment (The Body)

Orthos can now control the physical world.

### Docker Deployment
```bash
# Build and run the platform
docker-compose up -d

# Access the solver via TCP
nc localhost 7777

# Or via WebSocket
wscat -c ws://localhost:7778
```

### Home Assistant Integration

Connect to your smart home:

1. Copy `.env.example` to `.env`
2. Add your Home Assistant token
3. Edit `orthos-hass/rules/entities.yaml` with your entities
4. Run `docker-compose up -d`

**The Magic**: Lights turn on not because of a rule, but because *darkness violates the Law of Comfort*.

```
State: LightLevel=5, Occupancy=true
Law: Occupancy implies LightLevel >= 30
Solver: UNSAT → Lights must be ON
Result: light.living_room → turn_on
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ORTHOS PLATFORM                          │
├─────────────────────────────────────────────────────────────┤
│  std/                                                       │
│  ├── physics.orth      # Physical laws                      │
│  └── economics.orth    # Economic laws                      │
├─────────────────────────────────────────────────────────────┤
│  orthos-daemon/        # Core solver (Z3-backed)            │
│  orthos-bridge/        # TCP/WebSocket network access       │
│  orthos-hass/          # Home Assistant embodiment          │
├─────────────────────────────────────────────────────────────┤
│  docker-compose.yml    # One-command deployment             │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Use Standard Library
```orthos
import "std/physics.orth" as Physics

Boundary MyPhysics <-> {
    // Ball collision using std/physics
    Flux result = Physics.Collision1D(
        massA=2, velA_before=5,
        massB=1, velB_before=0
    ).velB_after
}
```

### Deploy to Docker
```bash
docker-compose build
docker-compose up -d
```

### Connect to Home Assistant
```bash
cp .env.example .env
# Edit .env with HASS_TOKEN
# Edit orthos-hass/rules/entities.yaml
docker-compose up -d orthos-hass
```

## Explainability (The Dashboard)

The daemon now explains *why* it made decisions:

```bash
echo '{"method":"audit","id":1}' | orthos-daemon
```

```json
{
  "goals": {
    "Comfort.TempMin": {"satisfied": true, "weight": 10, "cost": 0},
    "Economy.SavePower": {"satisfied": false, "weight": 5, "cost": 5}
  },
  "total_cost": 5
}
```

**Translation**: *"Heater is ON because Comfort (weight 10) was prioritized over Economy (weight 5)."*

## The Vision

**Before**: Every user defines Time, Physics, Comfort from scratch.

**After**: 
- `import "std/physics.orth" as Physics` → Start constraining objects
- `docker-compose up` → Solver controls your home
- Darkness becomes *ontologically impossible* when you're home
- The system *explains* why it turned on the lights
