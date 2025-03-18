class ThreadPool:
    """A Rust-managed thread pool."""
    def __init__(self, num_threads: int) -> None:
        """Construct a new ThreadPool with the given number of threads."""
