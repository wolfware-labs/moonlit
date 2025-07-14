using JetBrains.Annotations;

namespace Wolfware.Moonlit.Plugins.Pipelines;

[PublicAPI]
public sealed class MiddlewareOutput
{
  private readonly Dictionary<string, object?> _data;

  public MiddlewareOutput(IReadOnlyDictionary<string, object?>? initialData = null)
  {
    this._data = new Dictionary<string, object?>(initialData ?? new Dictionary<string, object?>());
  }

  public void Add<T>(string key, T value)
  {
    ArgumentNullException.ThrowIfNull(key, nameof(key));

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

    this._data[key] = value;
  }

  public Dictionary<string, object?> ToDictionary(string scope)
  {
    return this._data.ToDictionary(
      kvp => $"output:{scope}:{kvp.Key}",
      kvp => kvp.Value
    );
  }
}
