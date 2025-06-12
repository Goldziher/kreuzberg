from typing import Any

from PIL.Image import Image

class PaddleOCR:
    def __init__(
        self,
        use_angle_cls: bool = True,
        lang: str = "en",
        use_gpu: bool = False,
        gpu_mem: int = 8000,
        image_dir: str | None = None,
        det_algorithm: str = "DB",
        det_model_dir: str | None = None,
        det_limit_side_len: int = 960,
        det_limit_type: str = "min",
        det_db_thresh: float = 0.3,
        det_db_box_thresh: float = 0.5,
        det_db_unclip_ratio: float = 2.0,
        max_batch_size: int = 10,
        use_dilation: bool = False,
        det_db_score_mode: str = "fast",
        rec_algorithm: str = "CRNN",
        rec_model_dir: str | None = None,
        rec_image_shape: str = "3,32,320",
        rec_batch_size: int = 6,
        rec_char_dict_path: str | None = None,
        use_space_char: bool = True,
        drop_score: float = 0.5,
        use_mp: bool = False,
        total_process_num: int = 1,
        enable_mkldnn: bool = False,
        use_zero_copy_run: bool = False,
        device: str = "cpu",
    ) -> None: ...
    def ocr(
        self,
        img: str | Image,
        cls: bool = True,
        det: bool = True,
        rec: bool = True,
        **kwargs: Any,
    ) -> list[list[list[Any]]]: ...
