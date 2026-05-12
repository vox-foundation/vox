import Capacitor
import Foundation

@objc(VoxSherpaTranscribePlugin)
public class VoxSherpaTranscribePlugin: CAPPlugin {
    @objc func transcribe(_ call: CAPPluginCall) {
        AppleSpeechBackend.shared.transcribe { result in
            switch result {
            case .success(let (text, confidence)):
                call.resolve(["text": text, "confidence": confidence])
            case .failure(let err):
                call.reject(err.localizedDescription)
            }
        }
    }
}
