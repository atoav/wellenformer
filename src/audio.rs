use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia_core::audio::SampleBuffer;
use std::io;
use std::path::PathBuf;


// fn main() {
//     // Get the first command line argument.
//     let args: Vec<String> = std::env::args().collect();
//     let path = PathBuf::from(args.get(1).expect("file path not provided"));

//     let (channels, samples) = read_audio(&path);
//     println!("Read {} samples ({} Channels)", samples.len(), channels);

//     println!("Last Sample: {:?}", samples.last().unwrap());
// }


pub fn read_audio(path: &PathBuf) -> (usize, Vec<f32>) {
    // Open the media source.
    let src = std::fs::File::open(&path).expect("failed to open media");
    
    // Create a probe hint using the file's extension. [Optional]
    let mut hint = Hint::new();
    match path.extension() {
        Some(ext) => {
            hint.with_extension(&ext.to_string_lossy());
        },
        _ => ()
    }

    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(src), Default::default());


    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    let mut samples: Vec<f32> = vec![];
    let mut channels = 0;

    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                // The track list has been changed. Re-examine it and create a new set of decoders,
                // then restart the decode loop. This is an advanced feature and it is not
                // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                // for chained OGG physical streams.
                unimplemented!();
            }
            Err(err) => {
                // A unrecoverable error occured, halt decoding.
                match err {
                    Error::IoError(e) => {
                        match e.kind() {
                            io::ErrorKind::UnexpectedEof => break,
                            _ => {
                                panic!("{}", e)
                            }
                        }
                    },
                    _ => panic!("{}", err)
                }
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            format.metadata().pop();
            // Consume the new metadata at the head of the metadata queue.
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Create a sample buffer that matches the parameters of the decoded audio buffer.
                let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
                channels = decoded.spec().channels.count();

                // Copy the contents of the decoded audio buffer into the sample buffer whilst performing
                // any required conversions.
                sample_buf.copy_interleaved_ref(decoded);

                // The interleaved f32 samples can be accessed as follows.
                for sample in sample_buf.samples() {
                    // println!("{:?}", sample);
                    samples.push(sample.clone());
                }
                // samples.append();
            }
            Err(Error::IoError(_e)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                eprintln!("IO-Error");
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                eprintln!("Decode-Error");
                continue;
            }
            Err(err) => {
                // An unrecoverable error occured, halt decoding.
                panic!("{:?}", err);
            }
        }
    }
    return (channels, samples)
}

