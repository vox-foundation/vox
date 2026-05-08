import Foundation
import Capacitor

/// iOS: stub until Sherpa-ONNX (or Apple Speech) assets are packaged — keeps Capacitor contract stable for CI/simulator.
@objc(VoxSherpaTranscribePlugin)
public class VoxSherpaTranscribePlugin: CAPPlugin {

    @objc func transcribe(_ call: CAPPluginCall) {
        call.reject("iOS on-device STT not bundled in this tree — add onnxruntime + sherpa assets or wire Apple Speech (see plugin README).")
    }
}
