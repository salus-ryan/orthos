# ORTHOS Home Assistant Integration

**Embodiment Layer** - Connect the constraint solver to the physical world.

## Concept

The solver doesn't "decide" to turn on lights. It *realizes* that darkness violates the Law of Comfort, and lights become a **necessary consequence**.

```
Current State: LightLevel = 5, Occupancy = true
Law: Occupancy == true implies LightLevel >= 30
Solver: UNSAT without Lights = true
Result: Lights turn on
```

## Setup

### 1. Configure Home Assistant

Get a long-lived access token from your Home Assistant instance:
- Profile → Long-Lived Access Tokens → Create Token

### 2. Configure Entity Mappings

Edit `rules/entities.yaml` to map your actual Home Assistant entities:

```yaml
sensors:
  Temperature: sensor.your_temperature_sensor
  LightLevel: sensor.your_light_sensor
  Humidity: sensor.your_humidity_sensor
  Occupancy: binary_sensor.your_motion_sensor

actuators:
  Heater: climate.your_thermostat
  Cooler: climate.your_ac
  Lights: light.your_main_light
  Humidifier: switch.your_humidifier
```

### 3. Set Environment Variables

```bash
export HASS_URL="http://homeassistant.local:8123"
export HASS_TOKEN="your_long_lived_token"
export COMFORT_TEMP_MIN=20
export COMFORT_TEMP_MAX=24
export COMFORT_LIGHT_MIN=30
```

### 4. Run with Docker Compose

```bash
docker-compose up -d
```

## How It Works

1. **Sensor Loop**: Every 10 seconds, read sensor states from Home Assistant
2. **Constraint Injection**: Feed current state to ORTHOS as constraints
3. **Solve**: ORTHOS determines what actuator states satisfy the Laws of Comfort
4. **Actuate**: Apply the solution to Home Assistant entities

## Customizing Comfort Laws

Edit `rules/comfort.orth` to define your own laws:

```orthos
Boundary Comfort <-> {
    Flux Temperature : Int
    Flux Heater : Bool
    
    // Your custom law
    Law MyComfort : Temperature >= 22
    Law HeaterEffect : Heater == true implies Temperature >= 22
}
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Home Assistant │────▶│  orthos-hass    │────▶│  orthos-bridge  │
│    (Sensors)    │     │   (Controller)  │     │   (WebSocket)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
┌─────────────────┐                             ┌───────▼─────────┐
│  Home Assistant │◀────────────────────────────│  orthos-daemon  │
│   (Actuators)   │      Actuator Commands      │    (Solver)     │
└─────────────────┘                             └─────────────────┘
```

## Example: The Dark Room

**Scenario**: It's 8 PM, someone is home, but the lights are off.

```
Sensors:
  - TimeOfDay: 20
  - Occupancy: true
  - LightLevel: 0

Laws:
  - Occupancy == true implies LightLevel >= 30  (VIOLATED)
  - Lights == true implies LightLevel >= 40

Solver realizes:
  - Current state is UNSAT
  - Setting Lights = true makes it SAT

Action:
  - light.living_room_main → turn_on
```

The lights turn on not because of a rule that says "if dark, turn on lights" but because **darkness is ontologically impossible** when someone is home.
