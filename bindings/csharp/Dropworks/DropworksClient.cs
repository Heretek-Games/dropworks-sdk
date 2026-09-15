using System.Runtime.InteropServices;

namespace Dropworks;

public enum DropworksStatus
{
    Ok = 0,
    InvalidArgument = 1,
    Network = 2,
    Unauthorized = 3,
    Server = 4,
    NotSignedIn = 5,
    Internal = 6,
    NotImplemented = 7,
}

public sealed class DropworksException : Exception
{
    public DropworksStatus Status { get; }

    public DropworksException(DropworksStatus status, string? message)
        : base(message ?? DropworksNative.StatusString((int)status))
    {
        Status = status;
    }
}

public sealed class DropworksSession : IDisposable
{
    private IntPtr _handle;

    internal DropworksSession(IntPtr handle) => _handle = handle;

    internal IntPtr Handle => _handle;

    public string AppId => Read(DropworksNative.dropworks_session_app_id(_handle));

    public string UserId => Read(DropworksNative.dropworks_session_user_id(_handle));

    private static string Read(IntPtr pointer) =>
        pointer == IntPtr.Zero ? string.Empty : Marshal.PtrToStringUTF8(pointer) ?? string.Empty;

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            DropworksNative.dropworks_session_destroy(_handle);
            _handle = IntPtr.Zero;
        }
    }
}

/// <summary>Managed client for the Dropworks C ABI (<c>libdropworks</c>).</summary>
public sealed class DropworksClient : IDisposable
{
    private IntPtr _handle;

    public DropworksClient(string baseUrl, uint timeoutMs = 0)
    {
        var config = new DropworksNative.Config { BaseUrl = baseUrl, TimeoutMs = timeoutMs };
        var status = DropworksNative.dropworks_client_create(in config, out _handle);
        if (status != DropworksNative.Ok)
        {
            throw new DropworksException((DropworksStatus)status, null);
        }
    }

    /// <summary>Raw native handle, for advanced or diagnostic use.</summary>
    public IntPtr Handle => _handle;

    public DropworksSession SignIn(string appId, string authToken)
    {
        EnsureAlive();
        var status = DropworksNative.dropworks_sign_in(_handle, appId, authToken, out var session);
        if (status != DropworksNative.Ok)
        {
            throw new DropworksException((DropworksStatus)status, LastError());
        }
        return new DropworksSession(session);
    }

    public bool UnlockAchievement(DropworksSession session, string achievementId)
    {
        EnsureAlive();
        var status = DropworksNative.dropworks_unlock_achievement(
            _handle,
            session.Handle,
            achievementId,
            out var unlocked);
        if (status != DropworksNative.Ok)
        {
            throw new DropworksException((DropworksStatus)status, LastError());
        }
        return unlocked != 0;
    }

    public bool SubmitScore(DropworksSession session, string leaderboardKey, double score)
    {
        EnsureAlive();
        var status = DropworksNative.dropworks_submit_score(
            _handle,
            session.Handle,
            leaderboardKey,
            score,
            out var improved);
        if (status != DropworksNative.Ok)
        {
            throw new DropworksException((DropworksStatus)status, LastError());
        }
        return IsImproved(improved);
    }

    /// <summary>
    /// Maps the native tri-state <c>improved</c> out-parameter to a boolean:
    /// only <c>1</c> is <c>true</c>; <c>0</c> and
    /// <see cref="DropworksNative.BoolUnknown"/> (<c>-1</c>, the server did not
    /// report the field) are <c>false</c>.
    /// </summary>
    public static bool IsImproved(int improved) => improved == 1;

    public void SetPresence(DropworksSession session, string status, string? gameId = null)
    {
        EnsureAlive();
        var code = DropworksNative.dropworks_set_presence(_handle, session.Handle, gameId, status);
        if (code != DropworksNative.Ok)
        {
            throw new DropworksException((DropworksStatus)code, LastError());
        }
    }

    public string? LastError() => DropworksNative.LastError(_handle);

    private void EnsureAlive()
    {
        if (_handle == IntPtr.Zero)
        {
            throw new ObjectDisposedException(nameof(DropworksClient));
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            DropworksNative.dropworks_client_destroy(_handle);
            _handle = IntPtr.Zero;
        }
    }
}
