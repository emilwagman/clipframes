// squeeze in.mov out.mp4 start end width kbps : re-encode a clip to H.264 at a set bitrate, no audio.
import AVFoundation
let a = CommandLine.arguments
let src = AVURLAsset(url: URL(fileURLWithPath: a[1]))
let out = URL(fileURLWithPath: a[2])
let start = Double(a[3])!, end = Double(a[4])!, width = Double(a[5])!, kbps = Int(a[6])!
try? FileManager.default.removeItem(at: out)
let sem = DispatchSemaphore(value: 0)
Task {
    let track = try await src.loadTracks(withMediaType: .video)[0]
    let size = try await track.load(.naturalSize)
    let h = (size.height * width / size.width / 2).rounded() * 2
    let reader = try AVAssetReader(asset: src)
    reader.timeRange = CMTimeRange(start: CMTime(seconds: start, preferredTimescale: 600), end: CMTime(seconds: end, preferredTimescale: 600))
    let ro = AVAssetReaderTrackOutput(track: track, outputSettings: [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange])
    reader.add(ro)
    let writer = try AVAssetWriter(outputURL: out, fileType: .mp4)
    writer.shouldOptimizeForNetworkUse = true
    let wi = AVAssetWriterInput(mediaType: .video, outputSettings: [AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: Int(width), AVVideoHeightKey: Int(h), AVVideoScalingModeKey: AVVideoScalingModeResizeAspect,
        AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: kbps * 1000, AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel, AVVideoExpectedSourceFrameRateKey: 30]])
    wi.expectsMediaDataInRealTime = false
    writer.add(wi)
    guard reader.startReading() else { print("reader:", reader.error as Any); exit(1) }
    guard writer.startWriting() else { print("writer:", writer.error as Any); exit(1) }
    writer.startSession(atSourceTime: CMTime(seconds: start, preferredTimescale: 600))
    var last = CMTime.negativeInfinity
    wi.requestMediaDataWhenReady(on: DispatchQueue(label: "w")) {
        while wi.isReadyForMoreMediaData {
            guard let s = ro.copyNextSampleBuffer() else { wi.markAsFinished(); writer.finishWriting { sem.signal() }; return }
            let t = CMSampleBufferGetPresentationTimeStamp(s)
            if (t - last).seconds < 1.0 / 30 { continue } // 60 -> 30 fps
            last = t
            wi.append(s)
        }
    }
}
sem.wait()
print("done")
