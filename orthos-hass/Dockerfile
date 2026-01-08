# ORTHOS Home Assistant Integration
# Connects constraint solver to real-world actuators

FROM python:3.11-slim

WORKDIR /app

# Install dependencies
COPY orthos-hass/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application
COPY orthos-hass/src/ ./src/
COPY orthos-hass/rules/ ./rules/

RUN useradd -m -u 1000 orthos
USER orthos

ENTRYPOINT ["python", "-m", "src.main"]
