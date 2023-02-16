# wellenformer



![](images/aftermath.png)

This is a Rust-based CLI tool with the sole purpose of rendering audio files into (rectified) waveformsrepo.



## Features

- Oversampling (takes longer and needs more memory, but will result in a waveform with more detail)
- Colors can be adjusted to taste
- Transparent fore- and backgrounds possible
- Option to normalize audio
- Reads all kind of formats (wav, mp3, aac, flac, ...)



## Installation

1. Clone this repository somewhere
2. Ensure Rust is installed on your system
3. From within the repo run `cargo build --release`
4. Take the resulting `wellenformer` binary from the `target/release` directory and copy it somewhere into you path