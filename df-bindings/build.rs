fn main() {
    windows::build! {
        Windows::Foundation::*,
        Windows::Storage::Streams::Buffer,
        Windows::Devices::Midi::*,
        Windows::Win32::System::WinRT::IMemoryBufferByteAccess,
    }
}
