//! Visual source modules for clasSCII (image, video, procedural).

pub mod folder_batch;
pub mod image;
pub mod resize;

#[cfg(feature = "procedural")]
pub mod procedural;
#[cfg(feature = "video")]
pub mod video;
