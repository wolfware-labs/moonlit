namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Represents the configuration for a plugin, including its name, location, and additional settings.
/// </summary>
/// <remarks>
/// This class serves as a configuration object for defining plugin-related metadata and settings.
/// The configuration details include:
/// - The name of the plugin.
/// - The URI pointing to the plugin's location.
/// - A dictionary of arbitrary key-value pairs for customizing plugin behavior.
/// </remarks>
public sealed class PluginConfiguration
{
  /// <summary>
  /// Gets or sets the name of the plugin.
  /// </summary>
  /// <remarks>
  /// The name uniquely identifies the plugin and is used as a key in contexts like plugin management and configuration.
  /// This property is required and must be non-empty to ensure proper identification of the plugin.
  /// </remarks>
  public string Name { get; set; } = string.Empty;

  /// <summary>
  /// Gets or sets the URI pointing to the plugin's physical or remote location.
  /// </summary>
  /// <remarks>
  /// This property represents the path or address where the plugin is stored. It may point to a local file,
  /// a network location, or a remote URL depending on the application's requirements. The URI must be syntactically valid
  /// and resolvable by the underlying plugin loader system.
  /// </remarks>
  public Uri Url { get; set; } = new("file://plugin.dll");

  /// <summary>
  /// Gets or sets the key-value pair configuration settings for the plugin.
  /// </summary>
  /// <remarks>
  /// This property represents an extensible dictionary of configuration parameters
  /// that can be used to customize the behavior of a plugin.
  /// Keys are strings that define specific configuration options,
  /// and their respective values can be defined as nullable strings.
  /// These settings enable flexible adjustment of plugin attributes during initialization or runtime.
  /// </remarks>
  public Dictionary<string, object?> Configuration { get; set; } = [];
}
