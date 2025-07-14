using JetBrains.Annotations;

namespace Wolfware.Moonlit.Plugins.Pipelines;

/// <summary>
/// Represents the context within which the release pipeline and its associated middleware execute.
/// </summary>
/// <remarks>
/// The <see cref="ReleaseContext"/> provides necessary data and configuration to the release pipeline,
/// such as cancellation support, logging, and information about the working directory for the current execution.
/// </remarks>
[PublicAPI]
public sealed record ReleaseContext
{
  /// <summary>
  /// Gets or sets the cancellation token that is used to observe cancellation requests
  /// during the release pipeline execution.
  /// </summary>
  /// <remarks>
  /// This property enables cooperative cancellation between various components
  /// of the release process. It is typically monitored by tasks or operations
  /// that support cancellation, ensuring timely termination when a cancellation
  /// request is made.
  /// The provided <see cref="System.Threading.CancellationToken"/> can be used to check
  /// for cancellation state or trigger appropriate error handling mechanisms
  /// like throwing an <see cref="System.OperationCanceledException"/>.
  /// </remarks>
  public CancellationToken CancellationToken { get; init; }

  /// <summary>
  /// Gets the working directory for the release operation.
  /// </summary>
  /// <remarks>
  /// This property specifies the base directory path used for the release processes.
  /// It typically represents the root directory of the repository or project on which
  /// the operations, such as tagging or retrieving metadata, are performed. The default
  /// value is the current working directory of the environment.
  /// </remarks>
  public string WorkingDirectory { get; init; } = Environment.CurrentDirectory;
}
