#import <Foundation/Foundation.h>
#import <Capacitor/Capacitor.h>

CAP_PLUGIN(VoxSherpaTranscribePlugin, "VoxSherpaTranscribe",
           CAP_PLUGIN_METHOD(transcribe, CAPPluginReturnPromise);
)
