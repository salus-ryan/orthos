"""
ORTHOS Home Assistant Integration
Connects the constraint solver to real-world actuators.

The solver realizes "Darkness violates the Law of Comfort" and lights turn on.
"""

import asyncio
import json
import logging
import os
from pathlib import Path

import aiohttp
import websockets
import yaml

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('orthos-hass')


class HomeAssistantClient:
    """Client for Home Assistant REST API."""
    
    def __init__(self, url: str, token: str):
        self.url = url.rstrip('/')
        self.token = token
        self.headers = {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json',
        }
    
    async def get_state(self, entity_id: str) -> dict:
        """Get current state of an entity."""
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f'{self.url}/api/states/{entity_id}',
                headers=self.headers
            ) as resp:
                if resp.status == 200:
                    return await resp.json()
                else:
                    logger.error(f"Failed to get state for {entity_id}: {resp.status}")
                    return {}
    
    async def call_service(self, domain: str, service: str, entity_id: str, **kwargs) -> bool:
        """Call a Home Assistant service."""
        data = {'entity_id': entity_id, **kwargs}
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f'{self.url}/api/services/{domain}/{service}',
                headers=self.headers,
                json=data
            ) as resp:
                success = resp.status == 200
                if success:
                    logger.info(f"Called {domain}.{service} on {entity_id}")
                else:
                    logger.error(f"Service call failed: {resp.status}")
                return success
    
    async def turn_on(self, entity_id: str, **kwargs) -> bool:
        """Turn on an entity."""
        domain = entity_id.split('.')[0]
        return await self.call_service(domain, 'turn_on', entity_id, **kwargs)
    
    async def turn_off(self, entity_id: str) -> bool:
        """Turn off an entity."""
        domain = entity_id.split('.')[0]
        return await self.call_service(domain, 'turn_off', entity_id)
    
    async def set_temperature(self, entity_id: str, temperature: float) -> bool:
        """Set thermostat temperature."""
        return await self.call_service(
            'climate', 'set_temperature', entity_id,
            temperature=temperature
        )


class OrthosClient:
    """Client for ORTHOS Daemon via WebSocket bridge."""
    
    def __init__(self, url: str):
        self.url = url
        self.ws = None
        self.request_id = 0
    
    async def connect(self):
        """Connect to ORTHOS bridge."""
        self.ws = await websockets.connect(self.url)
        logger.info(f"Connected to ORTHOS at {self.url}")
    
    async def disconnect(self):
        """Disconnect from ORTHOS bridge."""
        if self.ws:
            await self.ws.close()
    
    async def send_request(self, method: str, params: dict = None) -> dict:
        """Send JSON-RPC request and get response."""
        self.request_id += 1
        request = {
            'jsonrpc': '2.0',
            'method': method,
            'id': self.request_id,
        }
        if params:
            request['params'] = params
        
        await self.ws.send(json.dumps(request))
        response = await self.ws.recv()
        return json.loads(response)
    
    async def initialize(self, source: str) -> dict:
        """Initialize with Orthos source code."""
        return await self.send_request('initialize', {'source': source})
    
    async def constrain(self, laws: list[str]) -> dict:
        """Add runtime constraints."""
        return await self.send_request('constrain', {'laws': laws})
    
    async def query(self, flux_names: list[str]) -> dict:
        """Query flux values."""
        return await self.send_request('query', {'flux': flux_names})
    
    async def checkpoint(self) -> dict:
        """Create checkpoint."""
        return await self.send_request('checkpoint')
    
    async def restore(self) -> dict:
        """Restore to checkpoint."""
        return await self.send_request('restore')


class ComfortLaw:
    """
    The Law of Comfort - defines what states are acceptable.
    Violations trigger actuator responses.
    """
    
    ORTHOS_TEMPLATE = '''
// Law of Comfort - Auto-generated from Home Assistant state
Boundary Comfort <-> {{
    // Current sensor readings
    Flux Temperature : Int
    Flux LightLevel : Int
    Flux Humidity : Int
    Flux Occupancy : Bool
    
    // Actuator states (solver determines these)
    Flux Heater : Bool
    Flux Cooler : Bool
    Flux Lights : Bool
    Flux Humidifier : Bool
    
    // === THE LAWS OF COMFORT ===
    
    // Temperature must be comfortable when occupied
    Law TempMin : Occupancy == true implies Temperature >= {temp_min}
    Law TempMax : Occupancy == true implies Temperature <= {temp_max}
    
    // Light must be adequate when occupied
    Law LightMin : Occupancy == true implies LightLevel >= {light_min}
    
    // Humidity should be reasonable
    Law HumidityMin : Humidity >= {humidity_min}
    Law HumidityMax : Humidity <= {humidity_max}
    
    // === PHYSICS OF ACTUATORS ===
    
    // Heater raises temperature
    Law HeaterEffect : Heater == true implies Temperature >= {temp_min}
    
    // Cooler lowers temperature  
    Law CoolerEffect : Cooler == true implies Temperature <= {temp_max}
    
    // Lights raise light level
    Law LightsEffect : Lights == true implies LightLevel >= {light_min}
    
    // Humidifier raises humidity
    Law HumidifierEffect : Humidifier == true implies Humidity >= {humidity_min}
    
    // === EFFICIENCY LAWS ===
    
    // Don't heat and cool simultaneously
    Law NoConflict : !(Heater == true && Cooler == true)
    
    // Don't run lights when unoccupied (soft goal)
    Goal EnergyEfficiency : Occupancy == false implies Lights == false @5
}}
'''
    
    def __init__(self, config: dict):
        self.temp_min = config.get('temp_min', 20)
        self.temp_max = config.get('temp_max', 24)
        self.light_min = config.get('light_min', 30)
        self.humidity_min = config.get('humidity_min', 30)
        self.humidity_max = config.get('humidity_max', 60)
    
    def generate_source(self) -> str:
        """Generate Orthos source code for comfort laws."""
        return self.ORTHOS_TEMPLATE.format(
            temp_min=self.temp_min,
            temp_max=self.temp_max,
            light_min=self.light_min,
            humidity_min=self.humidity_min,
            humidity_max=self.humidity_max,
        )


class OrthosHassController:
    """
    Main controller that bridges ORTHOS solver with Home Assistant.
    
    Flow:
    1. Read sensor states from Home Assistant
    2. Feed current state to ORTHOS as constraints
    3. ORTHOS solver determines required actuator states
    4. Apply actuator commands to Home Assistant
    """
    
    def __init__(self, hass: HomeAssistantClient, orthos: OrthosClient, config: dict):
        self.hass = hass
        self.orthos = orthos
        self.config = config
        self.comfort_law = ComfortLaw(config.get('comfort', {}))
        
        # Entity mappings
        self.sensors = config.get('sensors', {})
        self.actuators = config.get('actuators', {})
    
    async def initialize(self):
        """Initialize the ORTHOS solver with comfort laws."""
        source = self.comfort_law.generate_source()
        logger.info("Initializing ORTHOS with Comfort Laws...")
        result = await self.orthos.initialize(source)
        
        if 'error' in result:
            logger.error(f"ORTHOS initialization failed: {result['error']}")
            return False
        
        logger.info(f"ORTHOS ready. Flux: {result.get('result', {}).get('flux_list', [])}")
        return True
    
    async def read_sensors(self) -> dict:
        """Read current sensor values from Home Assistant."""
        readings = {}
        
        for flux_name, entity_id in self.sensors.items():
            state = await self.hass.get_state(entity_id)
            if state:
                value = state.get('state')
                # Convert to appropriate type
                if value in ('on', 'home', 'true'):
                    readings[flux_name] = True
                elif value in ('off', 'not_home', 'false'):
                    readings[flux_name] = False
                else:
                    try:
                        readings[flux_name] = int(float(value))
                    except (ValueError, TypeError):
                        logger.warning(f"Could not parse {flux_name}: {value}")
        
        logger.debug(f"Sensor readings: {readings}")
        return readings
    
    async def apply_constraints(self, readings: dict) -> dict:
        """Apply sensor readings as constraints and solve."""
        # Build constraint laws from readings
        laws = []
        for flux_name, value in readings.items():
            if isinstance(value, bool):
                laws.append(f"{flux_name} == {'true' if value else 'false'}")
            else:
                laws.append(f"{flux_name} == {value}")
        
        # Create checkpoint before constraining
        await self.orthos.checkpoint()
        
        # Apply constraints
        result = await self.orthos.constrain(laws)
        
        if result.get('result', {}).get('status') == 'UNSAT':
            logger.warning("Current state violates comfort laws!")
            core = result.get('result', {}).get('core', [])
            logger.warning(f"Conflict core: {core}")
            # Restore and try to find actuator solution
            await self.orthos.restore()
            return {'status': 'UNSAT', 'core': core}
        
        return {'status': 'SAT'}
    
    async def query_actuators(self) -> dict:
        """Query ORTHOS for required actuator states."""
        actuator_flux = list(self.actuators.keys())
        result = await self.orthos.query(actuator_flux)
        
        if 'error' in result:
            logger.error(f"Query failed: {result['error']}")
            return {}
        
        values = result.get('result', {}).get('values', {})
        violated = result.get('result', {}).get('violated_goals', [])
        
        if violated:
            logger.info(f"Violated goals (soft): {violated}")
        
        return values
    
    async def apply_actuators(self, states: dict):
        """Apply actuator states to Home Assistant."""
        for flux_name, value in states.items():
            entity_id = self.actuators.get(flux_name)
            if not entity_id:
                continue
            
            # Determine action based on value
            if isinstance(value, bool) or value in ('true', 'false', True, False):
                should_be_on = value in (True, 'true')
                if should_be_on:
                    await self.hass.turn_on(entity_id)
                else:
                    await self.hass.turn_off(entity_id)
            elif isinstance(value, (int, float)):
                # Numeric value - could be brightness, temperature, etc.
                domain = entity_id.split('.')[0]
                if domain == 'climate':
                    await self.hass.set_temperature(entity_id, float(value))
                elif domain == 'light':
                    await self.hass.turn_on(entity_id, brightness_pct=int(value))
    
    async def run_cycle(self):
        """Run one control cycle."""
        logger.info("--- Control Cycle ---")
        
        # 1. Read sensors
        readings = await self.read_sensors()
        
        # 2. Apply as constraints
        constraint_result = await self.apply_constraints(readings)
        
        # 3. Query actuator states
        actuator_states = await self.query_actuators()
        logger.info(f"Solver determined actuators: {actuator_states}")
        
        # 4. Apply to Home Assistant
        await self.apply_actuators(actuator_states)
        
        # 5. Restore for next cycle
        await self.orthos.restore()
    
    async def run_loop(self, interval: float = 10.0):
        """Run continuous control loop."""
        logger.info(f"Starting control loop (interval: {interval}s)")
        
        while True:
            try:
                await self.run_cycle()
            except Exception as e:
                logger.error(f"Control cycle error: {e}")
            
            await asyncio.sleep(interval)


def load_config() -> dict:
    """Load configuration from environment and files."""
    config = {
        'hass_url': os.environ.get('HASS_URL', 'http://homeassistant.local:8123'),
        'hass_token': os.environ.get('HASS_TOKEN', ''),
        'orthos_url': os.environ.get('ORTHOS_BRIDGE_URL', 'ws://localhost:7778'),
        'interval': float(os.environ.get('CONTROL_INTERVAL', '10')),
        'comfort': {
            'temp_min': int(os.environ.get('COMFORT_TEMP_MIN', '20')),
            'temp_max': int(os.environ.get('COMFORT_TEMP_MAX', '24')),
            'light_min': int(os.environ.get('COMFORT_LIGHT_MIN', '30')),
            'humidity_min': int(os.environ.get('COMFORT_HUMIDITY_MIN', '30')),
            'humidity_max': int(os.environ.get('COMFORT_HUMIDITY_MAX', '60')),
        },
        'sensors': {},
        'actuators': {},
    }
    
    # Load entity mappings from rules file
    rules_path = Path('/app/rules/entities.yaml')
    if rules_path.exists():
        with open(rules_path) as f:
            rules = yaml.safe_load(f)
            config['sensors'] = rules.get('sensors', {})
            config['actuators'] = rules.get('actuators', {})
    
    return config


async def main():
    """Main entry point."""
    config = load_config()
    
    if not config['hass_token']:
        logger.error("HASS_TOKEN environment variable required")
        return
    
    # Create clients
    hass = HomeAssistantClient(config['hass_url'], config['hass_token'])
    orthos = OrthosClient(config['orthos_url'])
    
    # Connect to ORTHOS
    try:
        await orthos.connect()
    except Exception as e:
        logger.error(f"Failed to connect to ORTHOS: {e}")
        return
    
    # Create controller
    controller = OrthosHassController(hass, orthos, config)
    
    # Initialize
    if not await controller.initialize():
        return
    
    # Run control loop
    try:
        await controller.run_loop(config['interval'])
    finally:
        await orthos.disconnect()


if __name__ == '__main__':
    asyncio.run(main())
