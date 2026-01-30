from ._decoder_runtime import Decoder
from .enums import Compression

class DecoderRegistry:
    """A registry holding multiple decoder methods."""
    def __init__(
        self, custom_decoders: dict[Compression | int, Decoder] | None = None
    ) -> None:
        """Construct a new decoder registry.

        By default, pure-Rust decoders will be used for any recognized and supported
        compression types. Only the supplied decoders will override Rust-native
        decoders.

        Args:
            custom_decoders: any custom decoder methods to use. This will be applied
                _after_ (and override) any default provided Rust decoders. Defaults to
                None.
        """
