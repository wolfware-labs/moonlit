namespace Wolfware.Moonlit.Plugins.Github.Extensions;

public static class DictionaryExtensions
{
  public static void AddRange<TKey, TValue>(
    this IDictionary<TKey, TValue?> dictionary,
    IEnumerable<KeyValuePair<TKey, TValue>> items)
  {
    ArgumentNullException.ThrowIfNull(dictionary, nameof(dictionary));
    ArgumentNullException.ThrowIfNull(items, nameof(items));

    foreach (var item in items)
    {
      dictionary[item.Key] = item.Value;
    }
  }
}
