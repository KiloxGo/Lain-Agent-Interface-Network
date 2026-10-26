FROM python:3.12-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt && useradd --uid 10001 --create-home knowledge
COPY lain_knowledge ./lain_knowledge
USER 10001:10001
EXPOSE 8788
CMD ["gunicorn", "lain_knowledge.api:create()", "--bind", "0.0.0.0:8788", "--workers", "1", "--threads", "4", "--timeout", "60", "--limit-request-line", "4096", "--limit-request-field_size", "8190", "--error-logfile", "-"]
