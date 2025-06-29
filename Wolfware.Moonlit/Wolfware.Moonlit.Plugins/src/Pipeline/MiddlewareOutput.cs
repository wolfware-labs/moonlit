using System.Text.Json;
using System.Text.Json.Serialization;
using JetBrains.Annotations;

namespace Wolfware.Moonlit.Plugins.Pipeline;

[PublicAPI]
public sealed class MiddlewareOutput
{
  private static readonly JsonSerializerOptions _serializerOptions = new()
  {
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull, PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
  };

  private readonly Dictionary<string, string?> _data;

  public MiddlewareOutput(IReadOnlyDictionary<string, string?>? initialData = null)
  {
    this._data = new Dictionary<string, string?>(initialData ?? new Dictionary<string, string?>());
  }

  public void Add<T>(string key, T value)
  {
    ArgumentNullException.ThrowIfNull(key, nameof(key));
    ArgumentNullException.ThrowIfNull(value, nameof(value));

    if (this._data.ContainsKey(key))
    {
      throw new ArgumentException($"Key '{key}' already exists in the middleware output.",
        nameof(key));
    }

    if (value is string strValue)
    {
      this._data[key] = strValue;
      return;
    }

    this._data[key] = JsonSerializer.Serialize(value, MiddlewareOutput._serializerOptions);
  }

  public Dictionary<string, string?> ToDictionary(string scope)
  {
    return this._data.ToDictionary(
      kvp => $"output:{scope}:{kvp.Key}",
      kvp => kvp.Value
    );
  }
}
