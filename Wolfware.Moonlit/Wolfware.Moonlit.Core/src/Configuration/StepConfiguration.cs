﻿namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Represents the configuration for an individual step in a process or pipeline.
/// This includes the step's name, associated plugin, middleware, and a dictionary
/// for additional configuration settings.
/// </summary>
public sealed class StepConfiguration
{
  /// <summary>
  /// Gets or sets the name of the step in the process or pipeline.
  /// The step name serves as an identifier for this specific step configuration.
  /// </summary>
  public string StepName { get; set; } = string.Empty;

  /// <summary>
  /// Gets or sets the name of the plugin associated with a step in the process or pipeline.
  /// The plugin name is used to reference specific functionality or behaviors provided by a plugin.
  /// </summary>
  public string PluginName { get; set; } = string.Empty;

  /// <summary>
  /// Gets or sets the name of the middleware associated with the step.
  /// The middleware name is used to identify and resolve the specific middleware
  /// component within the process or pipeline.
  /// </summary>
  public string MiddlewareName { get; set; } = string.Empty;

  /// <summary>
  /// Gets or sets a value indicating whether the process should continue execution of subsequent steps
  /// even if the current step fails. If set to true, the failure of this step will not interrupt
  /// the remaining pipeline, allowing subsequent steps to execute.
  /// </summary>
  public bool ContinueOnError { get; set; } = false;

  /// <summary>
  /// Gets or sets the condition that determines whether the step should be executed.
  /// The condition is typically an expression or a boolean value evaluated during the execution pipeline.
  /// </summary>
  public string? Condition { get; set; }

  /// <summary>
  /// Gets or sets the condition upon which the execution of the step should halt.
  /// This property defines a state or criteria where the pipeline is designed to stop processing further.
  /// </summary>
  public string? HaltIf { get; set; }

  /// <summary>
  /// Gets or sets the collection of configuration key-value pairs associated with the step.
  /// This dictionary stores additional customizable settings or parameters
  /// required for the step's execution.
  /// </summary>
  public Dictionary<string, object?> Configuration { get; set; } = new();
}
