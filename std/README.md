# ORTHOS Standard Library

Reusable constraint boundaries for common domains.

## Usage

Import and instantiate boundaries in your Orthos programs:

```orthos
// Use physics constraints
Boundary MySimulation <-> {
    Flux ball = Velocity(position=0, velocity=10, time=1).nextPosition
}
```

## Modules

### `physics.orth` - Physical Laws

- **Velocity** - 1D kinematics
- **Velocity2D** - 2D motion
- **Mass** - Newton's Second Law (F=ma)
- **Momentum** - Conservation of momentum
- **Collision1D** - Elastic collisions
- **InelasticCollision** - Perfectly inelastic collisions
- **Gravity** - Free fall kinematics
- **Energy** - Kinetic and potential energy
- **Spring** - Hooke's Law
- **Friction** - Static/kinetic friction
- **Pressure** - P = F/A
- **Density** - ρ = m/V
- **Wave** - v = fλ

### `economics.orth` - Economic Laws

- **Supply** - Supply curve
- **Demand** - Demand curve
- **MarketEquilibrium** - Price discovery
- **Transaction** - Buyer/seller exchange
- **Budget** - Consumer budget constraint
- **Profit** - Revenue - Cost
- **Production** - Production function
- **Inventory** - Stock management
- **Utility** - Consumer utility
- **ExchangeRate** - Currency conversion
- **Loan** - Simple interest
- **Tax** - Tax calculation
- **Auction** - Sealed-bid auction
- **Scarcity** - Resource allocation

## Examples

### Physics: Elastic Collision

```orthos
Boundary BilliardShot <-> {
    // Two balls collide
    Flux collision = Collision1D(
        massA=1, velA_before=10,
        massB=1, velB_before=0
    )
    
    // After collision, velocities swap (equal masses)
    Flux ballA_after = collision.velA_after  // = 0
    Flux ballB_after = collision.velB_after  // = 10
}
```

### Economics: Market Clearing

```orthos
Boundary CoffeeMarket <-> {
    // Find equilibrium price
    Flux market = MarketEquilibrium(
        baseSupply=0, supplySlope=2,
        baseDemand=100, demandSlope=1
    )
    
    // Solver finds: price=33, quantity=66
    Flux clearingPrice = market.equilibriumPrice
    Flux clearingQuantity = market.equilibriumQuantity
}
```

### Physics: Projectile Motion

```orthos
Boundary Projectile <-> {
    // Ball thrown upward
    Flux trajectory = Gravity(
        height=0, velocity=20, gravity=-10, time=2
    )
    
    // After 2 seconds: height=20, velocity=0 (apex)
    Flux peakHeight = trajectory.nextHeight
}
```
