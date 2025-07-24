namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Represents the configuration for a release process, containing settings for variables, plugins,
/// arguments, and staged steps to be executed in the pipeline.
/// </summary>
public sealed record ReleaseConfiguration
{
  /// <summary>
  /// Gets the name of the release configuration.
  /// This property represents a human-readable identifier for the configuration
  /// that can be used in logs, user interfaces, or other outputs to distinguish
  /// between different configurations.
  /// </summary>
  public string Name { get; init; } = string.Empty;

  /// <summary>
  /// Gets the arguments for the release process.
  /// This property defines a collection of key-value pairs that can be used to parameterize
  /// different aspects of the release configuration, allowing greater control and customization
  /// of the release pipeline.
  /// </summary>
  public Dictionary<string, string> Arguments { get; init; } = [];

  /// <summary>
  /// Gets the collection of variables defined for the release process configuration.
  /// These variables can be used as key-value pairs to parameterize or customize the behavior
  /// of different aspects of the release pipeline, such as steps, plugins, or stages.
  /// </summary>
  public Dictionary<string, string> Variables { get; init; } = [];

  /// <summary>
  /// Gets the collection of plugin configurations associated with the release process.
  /// This property defines the set of plugins to be utilized during the execution of the pipeline,
  /// detailing their metadata, locations, and any specific settings required for initialization.
  /// </summary>
  public PluginConfiguration[] Plugins { get; init; } = [];

  /// <summary>
  /// Represents the collection of stages in the release pipeline.
  /// Each stage is defined by a key-value pair, where the key is the name of the stage,
  /// and the value is an array of step configurations to be executed within that stage.
  /// Stages determine the logical grouping and execution order of steps in the pipeline.
  /// </summary>
  public Dictionary<string, StepConfiguration[]> Stages { get; init; } = [];
}
