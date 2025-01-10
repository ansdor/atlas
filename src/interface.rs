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
    /// Efficiently pack multiple textures into a single image
    Pack(PackArguments),
    /// Extract individual textures from a packed texture atlas
    Unpack(UnpackArguments),
    /// Evaluate the efficiency of different packing settings
    Query(QueryArguments),
    /// Pack multiple textures into a single, evenly tiled image
    Arrange(ArrangeArguments),
    /// Generate a LUT texture with an optional palette
    Lut(LutArguments),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ArrangeDirection {
    Horizontal,
    Vertical,
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

#[derive(Args, Debug)]
pub struct ArrangeArguments {
    /// The tile arrangement in NxN format, e.g. 4x4
    #[arg(required = true)]
    pub layout: String,
    /// Files or directories to be used as sources
    #[arg(required = true)]
    pub sources: Vec<String>,
    /// Output file to be generated
    #[arg(required = true)]
    pub output: String,
    /// Overwrite existing files
    #[arg(short = 'o')]
    pub overwrite: bool,
    /// Arrange images horizontally or vertically
    #[arg(short = 'd')]
    pub direction: Option<ArrangeDirection>,
    /// Quiet mode
    #[arg(short = 'q')]
    pub quiet: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LutArguments {
    /// Filename for the generated LUT texture
    #[arg(required = true)]
    pub output: String,
    /// Image file containing the colors to be used by the LUT
    #[arg(short = 'i')]
    pub image: Option<String>,
    /// Dimensions of the LUT cube, in pixels
    #[arg(short = 'd', default_value = "32")]
    pub dimensions: Option<usize>,
    /// Limit the maximum number of columns in the LUT
    #[arg(short = 'c', default_value = "16")]
    pub max_columns: Option<usize>,
    /// Overwrite existing files
    #[arg(short = 'o')]
    pub overwrite: bool,
    /// Quiet mode
    #[arg(short = 'q')]
    pub quiet: bool,
}
