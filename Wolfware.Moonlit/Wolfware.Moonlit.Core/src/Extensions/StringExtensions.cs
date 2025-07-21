namespace Wolfware.Moonlit.Core.Extensions;

public static class StringExtensions
{
  public static object ToClrType(this string value)
  {
    if (bool.TryParse(value, out var boolResult))
    {
      return boolResult;
    }

    if (int.TryParse(value, out var intResult))
    {
      return intResult;
    }

    if (double.TryParse(value, out var doubleResult))
    {
      return doubleResult;
    }

    if (DateTime.TryParse(value, out var dateTimeResult))
    {
      return dateTimeResult;
    }

    return value;
  }
}
