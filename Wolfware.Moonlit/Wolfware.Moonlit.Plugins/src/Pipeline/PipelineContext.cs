using JetBrains.Annotations;
using Microsoft.Extensions.Logging;

namespace Wolfware.Moonlit.Plugins.Pipeline;

/// <summary>
/// Represents the context for a pipeline execution.
/// </summary>
/// <remarks>
/// This class provides a mechanism for passing state and controlling the behavior of a pipeline.
/// It includes shared data, cancellation support, logging, and the ability to specify a working directory.
/// </remarks>
[PublicAPI]
public sealed record PipelineContext
{
  /// <summary>
  /// Gets or sets a dictionary that stores shared data for the pipeline execution.
  /// </summary>
  /// <remarks>
  /// This property allows adding, retrieving, and managing key-value pairs that are shared across pipeline stages.
  /// It is intended to store contextual or intermediate data that is useful for pipeline operations.
  /// </remarks>
  public Dictionary<string, object> Data { get; init; } = new();

  /// <summary>
  /// Gets or sets the <see cref="CancellationToken"/> that is used to monitor for cancellation requests during pipeline execution.
  /// </summary>
  /// <remarks>
  /// This property allows pipeline operations to respond to cancellation requests, enabling the graceful termination of execution.
  /// It can be particularly useful for long-running or asynchronous tasks within the pipeline context.
  /// </remarks>
  public CancellationToken CancellationToken { get; init; }

  /// <summary>
  /// Gets or sets the logger instance used for writing log messages during pipeline execution.
  /// </summary>
  /// <remarks>
  /// This property facilitates logging within the pipeline, enabling stages to record diagnostics,
  /// errors, or other relevant information. It provides a way to track the execution flow and aid
  /// in debugging or auditing the pipeline process.
  /// </remarks>
  public ILogger Logger { get; init; } = null!;

  /// <summary>
  /// Gets or sets the working directory for the pipeline execution.
  /// </summary>
  /// <remarks>
  /// This property specifies the directory path where operations within the pipeline are performed.
  /// It can be used to change or retrieve the current context for file or directory-related tasks during execution.
  /// </remarks>
  public string WorkingDirectory { get; init; } = Environment.CurrentDirectory;
}
