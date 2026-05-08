package com.vox.plugins.voxsherpatranscribe

import android.content.Intent
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin

/**
 * On-device transcription via Android [SpeechRecognizer] (offline packs when available).
 * Replace JNI with Sherpa-ONNX when model assets are bundled — keep the Capacitor contract stable.
 */
@CapacitorPlugin(name = "VoxSherpaTranscribe")
class VoxSherpaTranscribePlugin : Plugin() {

    @PluginMethod
    fun transcribe(call: PluginCall) {
        if (!SpeechRecognizer.isRecognitionAvailable(context)) {
            call.reject("Speech recognition is not available on this device")
            return
        }
        val speechRecognizer = SpeechRecognizer.createSpeechRecognizer(context)
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, false)
            putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
        }

        speechRecognizer.setRecognitionListener(object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {}
            override fun onBeginningOfSpeech() {}
            override fun onRmsChanged(rmsdB: Float) {}
            override fun onBufferReceived(buffer: ByteArray?) {}
            override fun onEndOfSpeech() {}

            override fun onError(error: Int) {
                speechRecognizer.destroy()
                call.reject("SpeechRecognizer error code: $error")
            }

            override fun onResults(results: Bundle?) {
                val matches = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                val confidences = results?.getFloatArray(SpeechRecognizer.CONFIDENCE_SCORES)
                val text = matches?.firstOrNull().orEmpty()
                val out = JSObject()
                out.put("text", text)
                if (confidences != null && confidences.isNotEmpty()) {
                    out.put("confidence", confidences[0].toDouble())
                }
                speechRecognizer.destroy()
                call.resolve(out)
            }

            override fun onPartialResults(partialResults: Bundle?) {}
            override fun onEvent(eventType: Int, params: Bundle?) {}
        })

        speechRecognizer.startListening(intent)
    }
}
