// Capture (specs/25): the native exception report and the V panic stream, written to staging
// files through quadra/native/sys.
module crash

import quadra.native.sys

fn arch_name() string {
	return $if arm64 { 'aarch64' } $else { 'x86_64' }
}

// native_header starts a native crash report; the filter appends the exception details.
fn native_header(version string) string {
	return 'OneTerm Native Crash Report\n================================\nVersion: ${version}\nOS: windows\nArchitecture: ${arch_name()}\nCapture: SetUnhandledExceptionFilter\n'
}

fn stderr_header(version string) string {
	return 'OneTerm Crash Report\n====================\nVersion: ${version}\nOS: windows\nArchitecture: ${arch_name()}\n\n'
}

// install opens the staging files and attaches the exception filter. Call it before the UI
// starts. With `redirect_stderr` true, stderr goes to the staging file for the process lifetime.
pub fn install(paths CrashCapturePaths, version string, redirect_stderr bool) ! {
	sys.install_crash_filter(sys.open_crash_staging(paths.native)!, native_header(version))!
	if redirect_stderr {
		sys.redirect_stderr(paths.stderr, stderr_header(version))!
	}
}
