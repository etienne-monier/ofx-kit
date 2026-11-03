//! CLI load support.
//!
//! This private module contains the load concerns for the OFX command-line tool.

use super::*;

pub(crate) struct Loaded {
    pub(crate) decoded: ofx_kit::DecodedSource,
    pub(crate) document: Result<OfxDocument, OfxError>,
}

pub(crate) fn load(path: &PathBuf) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoded = decode(&bytes);
    let document =
        parse_with_options(&decoded.text, &Default::default()).map(|parsed| parsed.document);
    Ok(Loaded { decoded, document })
}
