using System.Runtime.InteropServices;
using Dropworks;

var failures = 0;
void Check(bool condition, string what)
{
    if (!condition)
    {
        Console.Error.WriteLine($"FAIL: {what}");
        failures++;
    }
}

Check(
    DropworksNative.StatusString(DropworksNative.Ok) == "DROPWORKS_OK",
    "status_string(OK)");
Check(
    DropworksNative.StatusString(DropworksNative.ErrorUnauthorized)
        == "DROPWORKS_ERR_UNAUTHORIZED",
    "status_string(UNAUTHORIZED)");

using var client = new DropworksClient("https://drop.example.com", 1000);
Check(client.Handle != IntPtr.Zero, "client_create");

// Empty credentials are rejected by the native layer.
var signInStatus = DropworksNative.dropworks_sign_in(
    client.Handle, "", "", out var session);
Check(signInStatus == DropworksNative.ErrorInvalidArgument, "sign_in rejects empty app id");
Check(session == IntPtr.Zero, "no session on failure");

// A NULL session must be rejected, not crash.
var submitStatus = DropworksNative.dropworks_submit_score(
    client.Handle, IntPtr.Zero, "high-score", 1.0, out var improved);
Check(submitStatus == DropworksNative.ErrorInvalidArgument, "submit_score rejects null session");

var presenceStatus = DropworksNative.dropworks_set_presence(
    client.Handle, IntPtr.Zero, null, "in-game");
Check(presenceStatus == DropworksNative.ErrorInvalidArgument, "set_presence rejects null session");

Check(!string.IsNullOrEmpty(DropworksNative.LastError(client.Handle)), "last_error is set");

if (failures == 0)
{
    Console.WriteLine("dropworks C# binding smoke passed");
    return 0;
}

Console.Error.WriteLine($"{failures} check(s) failed");
return 1;
