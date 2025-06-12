from collections.abc import Sequence
from typing import Any, Literal, Protocol, TypeAlias

# Type aliases for PaddleOCR parameters
Language: TypeAlias = Literal["ch", "en", "french", "german", "korean", "japan"]
DetAlgorithm: TypeAlias = Literal["DB", "EAST", "SAST", "PSE", "FCE"]
RecAlgorithm: TypeAlias = Literal["CRNN", "SVTR_LCNet", "SVTR_LCNet_v2", "SVTR_LCNet_v3", "SVTR_Tiny"]

class PaddleOCR(Protocol):
    def __init__(
        self,
        use_angle_cls: bool = True,
        lang: Language = "en",
        use_gpu: bool = False,
        gpu_mem: int = 500,
        image_dir: str | None = None,
        det_algorithm: DetAlgorithm = "DB",
        det_model_dir: str | None = None,
        det_limit_side_len: int = 960,
        det_limit_type: Literal["min", "max"] = "max",
        det_db_thresh: float = 0.3,
        det_db_box_thresh: float = 0.5,
        det_db_unclip_ratio: float = 1.6,
        max_batch_size: int = 10,
        use_dilation: bool = False,
        det_db_score_mode: Literal["fast", "slow"] = "fast",
        rec_algorithm: RecAlgorithm = "CRNN",
        rec_model_dir: str | None = None,
        rec_image_shape: Sequence[int] = (3, 32, 320),
        rec_batch_size: int = 6,
        rec_char_dict_path: str | None = None,
        use_space_char: bool = True,
        drop_score: float = 0.5,
        use_mp: bool = False,
        total_process_num: int = 1,
        enable_mkldnn: bool = False,
        use_zero_copy_run: bool = False,
        device: str = "cpu",
        show_log: bool = True,
    ) -> None: ...
    def ocr(
        self,
        img: Any,
        cls: bool = True,
        det: bool = True,
        rec: bool = True,
        **kwargs: Any,
    ) -> list[list[tuple[list[list[float]], str, float]]]: ...
