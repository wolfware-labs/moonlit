using JetBrains.Annotations;

namespace Wolfware.Moonlit.Plugins.Pipeline;

/// Represents the outcome of a pipeline operation, encapsulating success, failure, and warning details.
[PublicAPI]
public sealed record PipelineResult
{
  /// Indicates whether the pipeline operation completed successfully.
  /// A value of `true` signifies that the operation succeeded without errors,
  /// whereas `false` indicates that the operation failed.
  public bool IsSuccessful { get; init; }

  /// Contains details about the error that occurred during the pipeline operation.
  /// When the pipeline operation fails, this property provides a descriptive message
  /// explaining the reason for the failure. If the operation is successful, this
  /// property will typically be null or empty.
  public string? ErrorMessage { get; init; }

  /// Holds additional data related to the outcome of a pipeline operation.
  /// This dictionary contains key-value pairs that can provide context or supplementary
  /// information about the operation's execution. Keys are strings and values are objects.
  public Dictionary<string, object> Data { get; init; } = new();

  /// Contains a collection of warning messages generated during the pipeline operation.
  /// These messages typically provide additional insights or non-critical issues that occurred
  /// while executing the pipeline but did not prevent the operation from succeeding.
  public List<string> Warnings { get; init; } = [];

  /// Creates a successful pipeline result with no additional data or warnings.
  /// <returns>
  /// A new instance of PipelineResult indicating a successful operation.
  /// </returns>
  public static PipelineResult Success() => new() {IsSuccessful = true};

  /// Creates a successful pipeline result with the specified additional data.
  /// <param name="data">
  /// A dictionary containing additional data to include in the result.
  /// </param>
  /// <returns>
  /// A new instance of PipelineResult indicating a successful operation with the provided data.
  /// </returns>
  public static PipelineResult Success(Dictionary<string, object> data) => new() {IsSuccessful = true, Data = data};

  /// Creates a failed pipeline result with the specified error message.
  /// <param name="errorMessage">
  /// A string representing the error message for the failed operation.
  /// </param>
  /// <returns>
  /// A new instance of PipelineResult indicating a failed operation with the provided error message.
  /// </returns>
  public static PipelineResult Failure(string errorMessage) =>
    new() {IsSuccessful = false, ErrorMessage = errorMessage};

  /// Creates a successful pipeline result with a specified warning message.
  /// <param name="warning">
  /// A string representing the warning message to include in the result.
  /// </param>
  /// <returns>
  /// A new instance of PipelineResult indicating a successful operation with the provided warning message.
  /// </returns>
  public static PipelineResult Warning(string warning) => new() {IsSuccessful = true, Warnings = {warning}};
}
