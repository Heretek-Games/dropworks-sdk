using System.Runtime.InteropServices;

namespace Dropworks;

/// <summary>
/// Raw P/Invoke surface for <c>libdropworks</c>. Prefer <see cref="DropworksClient"/>.
/// </summary>
public static class DropworksNative
{
    private const string Library = "dropworks";

    public const int Ok = 0;
    public const int ErrorInvalidArgument = 1;
    public const int ErrorNetwork = 2;
    public const int ErrorUnauthorized = 3;
    public const int ErrorServer = 4;
    public const int ErrorNotSignedIn = 5;
    public const int ErrorInternal = 6;
    public const int ErrorNotImplemented = 7;

    [StructLayout(LayoutKind.Sequential)]
    public struct Config
    {
        [MarshalAs(UnmanagedType.LPUTF8Str)]
        public string BaseUrl;

        public uint TimeoutMs;
    }

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern int dropworks_client_create(in Config config, out IntPtr client);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern void dropworks_client_destroy(IntPtr client);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern int dropworks_sign_in(
        IntPtr client,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string appId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string authToken,
        out IntPtr session);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern void dropworks_session_destroy(IntPtr session);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr dropworks_session_app_id(IntPtr session);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr dropworks_session_user_id(IntPtr session);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern int dropworks_unlock_achievement(
        IntPtr client,
        IntPtr session,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string achievementId,
        out int unlocked);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern int dropworks_submit_score(
        IntPtr client,
        IntPtr session,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string leaderboardKey,
        double score,
        out int improved);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern int dropworks_set_presence(
        IntPtr client,
        IntPtr session,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? gameId,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string status);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr dropworks_status_string(int status);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr dropworks_last_error(IntPtr client);

    public static string StatusString(int status) =>
        Marshal.PtrToStringUTF8(dropworks_status_string(status)) ?? "unknown";

    public static string? LastError(IntPtr client)
    {
        var pointer = dropworks_last_error(client);
        return pointer == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(pointer);
    }
}
