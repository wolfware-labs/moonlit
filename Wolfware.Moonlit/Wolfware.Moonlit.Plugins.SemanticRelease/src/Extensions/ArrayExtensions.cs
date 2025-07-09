namespace Wolfware.Moonlit.Plugins.SemanticRelease.Extensions;

public static class ArrayExtensions
{
  public static async Task<TResponse[]> ParallelForEachBatch<T, TResponse>(this T[] array, int batchSize,
    Func<T[], Task<TResponse>> action)
  {
    if (array.Length == 0)
    {
      return [];
    }

    var tasks = new List<Task<TResponse>>();
    for (int i = 0; i < array.Length; i += batchSize)
    {
      var batch = array.Skip(i).Take(batchSize).ToArray();
      tasks.Add(action(batch));
    }

    return await Task.WhenAll(tasks);
  }
}
