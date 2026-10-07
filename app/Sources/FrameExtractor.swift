import AVFoundation

enum FrameExtractor {
    struct Result {
        let duration: Double
        let count: Int
        let interval: Double
        let size: CGSize
    }

    /// Pulls evenly spaced frames: `fps` per second of video, capped at `maxFrames`.
    /// Each frame is taken from the middle of its slice, so the first and last
    /// frames are not the moments the recording started and stopped.
    static func extract(video: URL, into framesDir: URL, fps: Double, maxFrames: Int, width: Int) async throws -> Result {
        let asset = AVURLAsset(url: video)
        let duration = try await asset.load(.duration).seconds
        guard duration.isFinite, duration > 0 else {
            throw Shooter.Failure(errorDescription: "the recording has no length")
        }
        let count = max(1, min(max(1, maxFrames), Int((duration * max(0.1, fps)).rounded())))
        let interval = duration / Double(count)

        let generator = AVAssetImageGenerator(asset: asset)
        generator.appliesPreferredTrackTransform = true
        generator.requestedTimeToleranceBefore = .zero
        generator.requestedTimeToleranceAfter = .zero
        if width > 0 { generator.maximumSize = CGSize(width: width, height: width * 4) }

        try FileManager.default.createDirectory(at: framesDir, withIntermediateDirectories: true)
        var size = CGSize.zero
        for i in 0..<count {
            let t = (Double(i) + 0.5) * interval
            let (image, _) = try await generator.image(at: CMTime(seconds: t, preferredTimescale: 600))
            size = CGSize(width: image.width, height: image.height)
            try Shooter.writePNG(image, to: framesDir.appendingPathComponent(String(format: "%03d.png", i + 1)))
        }
        return Result(duration: duration, count: count, interval: interval, size: size)
    }

    /// The first frame taken at or after `time` (frames sit at the middle of their slice).
    static func frame(after time: Double, interval: Double, count: Int) -> Int {
        let i = Int(ceil(time / interval + 0.5))
        return min(max(1, i), count)
    }
}
