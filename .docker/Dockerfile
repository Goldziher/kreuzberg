# Build stage
FROM ghcr.io/astral-sh/uv:python3.13-bookworm AS builder
ARG EXTRAS=""
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV UV_LINK_MODE=copy
ENV UV_CACHE_DIR=/tmp/uv-cache

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy all necessary files for Rust build
COPY pyproject.toml uv.lock README.md ./
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY kreuzberg kreuzberg

# Install maturin and build Rust extension
# Use --compatibility linux to avoid manylinux issues in Docker
RUN --mount=type=cache,target=/tmp/uv-cache \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    uv pip install --system maturin && \
    maturin build --release --out dist --compatibility linux && \
    uv venv && \
    uv pip install dist/*.whl && \
    if [ -z "$EXTRAS" ]; then \
        uv sync --extra api --no-editable --no-dev --compile-bytecode; \
    else \
        extras_args="--extra api"; \
        for extra in $EXTRAS; do \
            extras_args="$extras_args --extra $extra"; \
        done; \
        uv sync $extras_args --no-editable --no-dev --compile-bytecode; \
    fi && \
    rm -rf /app/.venv/lib/python*/site-packages/**/__pycache__ && \
    find /app/.venv -type f -name "*.pyc" -delete && \
    find /app/.venv -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true && \
    find /app/.venv -type d -name "test" -exec rm -rf {} + 2>/dev/null || true && \
    rm -rf dist

# Runtime stage
FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim AS runtime
ARG EXTRAS=""
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV PATH="/app/.venv/bin:$PATH"

# Install runtime dependencies
# Languages included: English (default), Spanish, French, German, Italian, Portuguese,
# Chinese (simplified & traditional), Japanese, Arabic, Russian, Hindi ~keep
RUN apt-get update && apt-get install -y --no-install-recommends \
    pandoc \
    tesseract-ocr \
    tesseract-ocr-eng \
    tesseract-ocr-osd \
    tesseract-ocr-spa \
    tesseract-ocr-fra \
    tesseract-ocr-deu \
    tesseract-ocr-ita \
    tesseract-ocr-por \
    tesseract-ocr-chi-sim \
    tesseract-ocr-chi-tra \
    tesseract-ocr-jpn \
    tesseract-ocr-ara \
    tesseract-ocr-rus \
    tesseract-ocr-hin \
    libglib2.0-0 \
    libsm6 \
    libxext6 \
    libxrender-dev \
    libgomp1 \
    libgl1 \
    libglib2.0-0 \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Copy virtual environment from builder
COPY --from=builder /app/.venv /app/.venv
COPY --from=builder /app/kreuzberg /app/kreuzberg

# Create non-root user and cache directory
RUN groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser && \
    mkdir -p /app/.kreuzberg && \
    chown -R appuser:appuser /app

# Set default cache directory to prevent permission issues
ENV KREUZBERG_CACHE_DIR=/app/.kreuzberg

USER appuser
CMD ["litestar", "--app", "kreuzberg._api.main:app", "run", "--host", "0.0.0.0"]
