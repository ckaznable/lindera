#[cfg(feature = "embed-ipadic")]
use std::env;
#[cfg(feature = "compress")]
use std::ops::Deref;

use lindera_dictionary::LinderaResult;
#[cfg(feature = "compress")]
use lindera_dictionary::decompress::{CompressedData, decompress};
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::metadata::Metadata;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::loader::DictionaryLoader;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            // First check if this is compressed data by attempting to check aligned root
            let mut aligned = rkyv::util::AlignedVec::<16>::new();
            aligned.extend_from_slice(&$bytes[..]);
            match rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned) {
                Ok(compressed_data) => {
                    // Decompress it
                    match decompress(compressed_data) {
                        Ok(decompressed) => decompressed,
                        Err(_) => {
                            // Decompression failed, fall back to raw data
                            $bytes.to_vec()
                        }
                    }
                }
                Err(_) => {
                    // Not compressed data format, use as raw binary
                    $bytes.to_vec()
                }
            }
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

macro_rules! ipadic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "embed-ipadic"))]
        decompress_data!($name, &[], $filename);
    };
}

// Metadata-specific macro (skips compression/decompression processing)
macro_rules! ipadic_metadata {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "embed-ipadic")]
        const $name: &'static [u8] = include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path));
        #[cfg(not(feature = "embed-ipadic"))]
        const $name: &'static [u8] = &[];
    };
}

ipadic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-ipadic/char_def.bin",
    "char_def.bin"
);
ipadic_data!(CONNECTION_DATA, "/lindera-ipadic/matrix.mtx", "matrix.mtx");
ipadic_data!(DA_DATA, "/lindera-ipadic/dict.da", "dict.da");
ipadic_data!(VALS_DATA, "/lindera-ipadic/dict.vals", "dict.vals");
ipadic_data!(UNKNOWN_DATA, "/lindera-ipadic/unk.bin", "unk.bin");
ipadic_data!(
    WORDS_IDX_DATA,
    "/lindera-ipadic/dict.wordsidx",
    "dict.wordsidx"
);
ipadic_data!(WORDS_DATA, "/lindera-ipadic/dict.words", "dict.words");
ipadic_metadata!(
    METADATA_DATA,
    "/lindera-ipadic/metadata.json",
    "metadata.json"
);

pub fn load() -> LinderaResult<Dictionary> {
    // Load metadata from embedded binary data
    let metadata = Metadata::load(METADATA_DATA)?;

    #[cfg(feature = "compress")]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA.deref(),
                VALS_DATA.deref(),
                WORDS_IDX_DATA.deref(),
                WORDS_DATA.deref(),
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA.deref()),
            character_definition: CharacterDefinition::load(&CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(&UNKNOWN_DATA)?,
            metadata,
        })
    }
    #[cfg(not(feature = "compress"))]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA,
                VALS_DATA,
                WORDS_IDX_DATA,
                WORDS_DATA,
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA),
            character_definition: CharacterDefinition::load(CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(UNKNOWN_DATA)?,
            metadata,
        })
    }
}

pub struct EmbeddedIPADICLoader;

impl Default for EmbeddedIPADICLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedIPADICLoader {
    pub fn new() -> Self {
        Self
    }
}

impl DictionaryLoader for EmbeddedIPADICLoader {
    fn load(&self) -> LinderaResult<Dictionary> {
        load()
    }

    fn load_temporary(&self) -> LinderaResult<Dictionary> {
        load_temporary()
    }
}

/// Decompress embedded data or return raw bytes.
/// This function does not use static caching - it decompresses on every call.
#[cfg(feature = "compress")]
fn decompress_embedded_data(bytes: &[u8]) -> Vec<u8> {
    // First check if this is compressed data by attempting to check aligned root
    let mut aligned = rkyv::util::AlignedVec::<16>::new();
    aligned.extend_from_slice(bytes);
    match rkyv::from_bytes::<CompressedData, rkyv::rancor::Error>(&aligned) {
        Ok(compressed_data) => {
            // Decompress it
            match decompress(compressed_data) {
                Ok(decompressed) => decompressed,
                Err(_) => {
                    // Decompression failed, fall back to raw data
                    bytes.to_vec()
                }
            }
        }
        Err(_) => {
            // Not compressed data format, use as raw binary
            bytes.to_vec()
        }
    }
}

/// Load dictionary without static caching.
/// This function creates a new dictionary instance on every call,
/// decompressing data each time (if compression is enabled).
pub fn load_temporary() -> LinderaResult<Dictionary> {
    // Load metadata from embedded binary data
    let metadata = Metadata::load(METADATA_DATA)?;

    // Include raw bytes for temporary loading
    let char_def_bytes = include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/char_def.bin"
    ));
    let matrix_bytes = include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/matrix.mtx"
    ));
    let da_bytes = include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ipadic/dict.da"));
    let vals_bytes = include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.vals"
    ));
    let wordsidx_bytes = include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.wordsidx"
    ));
    let words_bytes = include_bytes!(concat!(
        env!("LINDERA_WORKDIR"),
        "/lindera-ipadic/dict.words"
    ));
    let unk_bytes = include_bytes!(concat!(env!("LINDERA_WORKDIR"), "/lindera-ipadic/unk.bin"));

    #[cfg(feature = "compress")]
    {
        let char_def_data = decompress_embedded_data(char_def_bytes);
        let matrix_data = decompress_embedded_data(matrix_bytes);
        let da_data = decompress_embedded_data(da_bytes);
        let vals_data = decompress_embedded_data(vals_bytes);
        let wordsidx_data = decompress_embedded_data(wordsidx_bytes);
        let words_data = decompress_embedded_data(words_bytes);
        let unk_data = decompress_embedded_data(unk_bytes);

        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                da_data,
                vals_data,
                wordsidx_data,
                words_data,
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(matrix_data),
            character_definition: CharacterDefinition::load(&char_def_data)?,
            unknown_dictionary: UnknownDictionary::load(&unk_data)?,
            metadata,
        })
    }
    #[cfg(not(feature = "compress"))]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                da_bytes,
                vals_bytes,
                wordsidx_bytes,
                words_bytes,
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(matrix_bytes),
            character_definition: CharacterDefinition::load(char_def_bytes)?,
            unknown_dictionary: UnknownDictionary::load(unk_bytes)?,
            metadata,
        })
    }
}
