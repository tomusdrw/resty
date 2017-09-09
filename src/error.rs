/// API error format
#[derive(Debug, Default, Serialize)]
pub struct Error {
  /// Error code
  pub code: u32,
  /// Error message
  pub message: String,
  /// Error details
  pub details: String,
}
