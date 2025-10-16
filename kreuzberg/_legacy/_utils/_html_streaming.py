from __future__ import annotations

_STREAMING_THRESHOLD_KB = 10
_LARGE_FILE_THRESHOLD_MB = 1
_DEFAULT_CHUNK_SIZE = 2048
_LARGE_FILE_CHUNK_SIZE = 4096

_STREAMING_THRESHOLD_BYTES = _STREAMING_THRESHOLD_KB * 1024
_LARGE_FILE_THRESHOLD_BYTES = _LARGE_FILE_THRESHOLD_MB * 1024 * 1024


def should_use_streaming(content_size: int) -> tuple[bool, int]:
    if content_size < 0:
        return False, _DEFAULT_CHUNK_SIZE

    if content_size > _STREAMING_THRESHOLD_BYTES:
        if content_size > _LARGE_FILE_THRESHOLD_BYTES:
            return True, _LARGE_FILE_CHUNK_SIZE
        return True, _DEFAULT_CHUNK_SIZE
    return False, _DEFAULT_CHUNK_SIZE
