use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate texture atlas from list of images
    Pack(PackArguments),
    /// Extract textures from a packed texture atlas
    Unpack(UnpackArguments),
    /// Evaluate different packing settings for efficiency
    Query(QueryArguments),
}

#[derive(Args, Debug)]
pub struct UnpackArguments {
    /// Text file describing the texture atlas to be unpacked
    #[arg(required = true)]
    pub source: String,
    /// Directory where the extracted images will go
    #[arg(required = true)]
    pub output_directory: String,
    /// Overwrite existing files
    #[arg(short = 'o')]
    pub overwrite: bool,
    /// Quiet mode
    #[arg(short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct QueryArguments {
    /// Files or directories to be used as sources for the query
    #[arg(required = true)]
    pub sources: Vec<String>,
    /// Space between the textures, in pixels
    #[arg(short = 's')]
    pub spacing: Option<u32>,
    /// Use a fixed size for the texture pages
    #[arg(short = 'p')]
    pub page_size: Option<String>,
    /// Don't merge duplicate images in the output
    #[arg(long = "no-dedup")]
    pub include_duplicates: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Args, Debug, Clone)]
pub struct PackArguments {
    /// Files or directories to be used as sources for the texture atlas
    #[arg(required = true)]
    pub sources: Vec<String>,
    /// File name for the generated files (e.g. 'foo' will generate 'foo.png' and 'foo.json')
    #[arg(required = true)]
    pub output: String,
    /// Overwrite existing files
    #[arg(short = 'o')]
    pub overwrite: bool,
    /// Space between the textures, in pixels
    #[arg(short = 's')]
    pub spacing: Option<u32>,
    /// Use a fixed size for the texture pages
    #[arg(short = 'p')]
    pub page_size: Option<String>,
    /// Description format
    #[arg(short = 'f')]
    pub format: Option<OutputFormat>,
    /// Quiet mode (nothing will be printed)
    #[arg(short = 'q')]
    pub quiet: bool,
    /// Pack rectangles by area instead of distance
    #[arg(long = "area")]
    pub pack_by_area: bool,
    /// Sort images by short side instead of long
    #[arg(long = "short")]
    pub short_side_sort: bool,
    /// Do not sort source images, pack them in the order they were provided
    #[arg(long = "unsorted")]
    pub unsorted: bool,
    /// Allow 90-degree rotation for more efficient packing
    #[arg(long = "rotate")]
    pub rotate: bool,
    /// Generate texture with power-of-two dimensions
    #[arg(long = "po2")]
    pub power_of_two: bool,
    /// Don't merge duplicate images in the output
    #[arg(long = "no-dedup")]
    pub include_duplicates: bool,
}
