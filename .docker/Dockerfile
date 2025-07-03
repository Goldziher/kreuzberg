FROM ghcr.io/astral-sh/uv:python3.13-bookworm as app
ARG EXTRAS=""
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE 1
ENV PYTHONUNBUFFERED 1
ENV UV_LINK_MODE=copy

RUN apt-get update && apt-get install -y --no-install-recommends \
    pandoc \
    tesseract-ocr \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*


COPY pyproject.toml uv.lock README.md ./
COPY kreuzberg kreuzberg

RUN uv sync --extra api${EXTRAS:+ --extra ${EXTRAS}} --no-editable --no-dev --compile-bytecode

RUN groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser
USER appuser
CMD ["litestar", "--app", "kreuzberg._api.main:app", "run", "--host", "0.0.0.0"]
