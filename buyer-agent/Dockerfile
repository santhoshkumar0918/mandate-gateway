FROM python:3.12-slim
WORKDIR /app
RUN pip install --no-cache-dir redis
COPY main.py worker.py ./
# Default: run as a continuous worker. Set REDIS_URL to enable queue mode.
ENV REDIS_URL=
CMD ["python", "worker.py"]
