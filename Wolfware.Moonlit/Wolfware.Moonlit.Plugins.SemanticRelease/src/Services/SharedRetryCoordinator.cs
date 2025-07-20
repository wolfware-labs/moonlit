namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public class SharedRetryCoordinator
{
  private readonly SemaphoreSlim _lock = new(1, 1);
  private DateTime _retryAvailableAt = DateTime.UtcNow;

  public async Task WaitIfRateLimitedAsync()
  {
    await _lock.WaitAsync();
    try
    {
      var now = DateTime.UtcNow;
      if (now < _retryAvailableAt)
      {
        var waitTime = _retryAvailableAt - now;
        Console.WriteLine($"Global backoff in place. Waiting {waitTime.TotalSeconds} seconds...");
        await Task.Delay(waitTime);
      }
    }
    finally
    {
      _lock.Release();
    }
  }

  public void SetGlobalRetryAfter(TimeSpan delay)
  {
    _retryAvailableAt = DateTime.UtcNow + delay;
  }
}
