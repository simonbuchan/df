fn main() {
    windows::build! {
        Windows::Foundation::*,

        Windows::Devices::Midi::*,
        Windows::Media::{
            AudioBuffer,
            AudioBufferAccessMode,
            AudioFrame,
        },
        Windows::Media::Audio::{
            AudioDeviceOutputNode,
            AudioFrameCompletedEventArgs,
            AudioFrameInputNode,
            AudioGraph,
            AudioGraphSettings,
            CreateAudioDeviceOutputNodeResult,
            CreateAudioGraphResult,
        },
        Windows::Media::MediaProperties::{
            AudioEncodingProperties,
        },
        Windows::Media::Render::AudioRenderCategory,
        Windows::Storage::Streams::Buffer,

        Windows::Win32::System::WinRT::{
            IMemoryBufferByteAccess,
        },
    }
}
