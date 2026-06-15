import { useCallback, useEffect, useRef, useState } from 'react';

const DEFAULT_DEVICE_ID = 'default';

export interface PreJoinDevice {
  deviceId: string;
  label: string;
}

interface PreviewTrackOptions {
  deviceId: string;
  enabled: boolean;
  hasMediaDevices: boolean;
  kind: 'audio' | 'video';
  onUnavailable: () => void;
  refreshDevices: () => Promise<void>;
}

const stopStream = (stream: MediaStream | null) => {
  stream?.getTracks().forEach((track) => track.stop());
};

const deviceConstraint = (deviceId: string) => {
  if (!deviceId || deviceId === DEFAULT_DEVICE_ID) {
    return true;
  }

  return { deviceId: { exact: deviceId } };
};

const mapDevices = (devices: MediaDeviceInfo[], kind: MediaDeviceKind) =>
  devices
    .filter(
      (device) =>
        device.kind === kind &&
        device.deviceId.length > 0 &&
        device.deviceId !== DEFAULT_DEVICE_ID,
    )
    .map((device, index) => ({
      deviceId: device.deviceId,
      label:
        device.label ||
        `${kind === 'audioinput' ? 'Microphone' : 'Camera'} ${index + 1}`,
    }));

const usePreviewTrack = ({
  deviceId,
  enabled,
  hasMediaDevices,
  kind,
  onUnavailable,
  refreshDevices,
}: PreviewTrackOptions) => {
  const [error, setError] = useState(false);
  const [stream, setStream] = useState<MediaStream | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  const replaceStream = useCallback((nextStream: MediaStream | null) => {
    setStream((currentStream) => {
      stopStream(currentStream);
      streamRef.current = nextStream;

      return nextStream;
    });
  }, []);

  useEffect(() => {
    if (!hasMediaDevices || !enabled) {
      replaceStream(null);
      return;
    }

    let isMounted = true;
    let pendingStream: MediaStream | null = null;

    navigator.mediaDevices
      .getUserMedia({
        audio: kind === 'audio' ? deviceConstraint(deviceId) : false,
        video: kind === 'video' ? deviceConstraint(deviceId) : false,
      })
      .then((nextStream) => {
        pendingStream = nextStream;
        if (!isMounted) {
          stopStream(nextStream);
          return;
        }

        setError(false);
        replaceStream(nextStream);
        void refreshDevices();
      })
      .catch(() => {
        if (!isMounted) {
          return;
        }

        setError(true);
        replaceStream(null);
        onUnavailable();
      });

    return () => {
      isMounted = false;
      stopStream(pendingStream);
    };
  }, [
    deviceId,
    enabled,
    hasMediaDevices,
    kind,
    onUnavailable,
    refreshDevices,
    replaceStream,
  ]);

  useEffect(() => {
    return () => stopStream(streamRef.current);
  }, []);

  return { error, stream };
};

export const usePreJoinMedia = () => {
  const hasMediaDevices =
    typeof navigator !== 'undefined' && !!navigator.mediaDevices?.getUserMedia;

  const [audioDevices, setAudioDevices] = useState<PreJoinDevice[]>([]);
  const [videoDevices, setVideoDevices] = useState<PreJoinDevice[]>([]);
  const [audioDeviceId, setAudioDeviceId] = useState(DEFAULT_DEVICE_ID);
  const [videoDeviceId, setVideoDeviceId] = useState(DEFAULT_DEVICE_ID);
  const [audioEnabled, setAudioEnabled] = useState(true);
  const [videoEnabled, setVideoEnabled] = useState(true);
  const [audioLevel, setAudioLevel] = useState(0);

  const refreshDevices = useCallback(async () => {
    if (!navigator.mediaDevices?.enumerateDevices) {
      return;
    }

    const devices = await navigator.mediaDevices.enumerateDevices();
    setAudioDevices(mapDevices(devices, 'audioinput'));
    setVideoDevices(mapDevices(devices, 'videoinput'));
  }, []);

  const disableUnavailableAudio = useCallback(() => {
    setAudioEnabled(false);
    setAudioLevel(0);
  }, []);

  const disableUnavailableVideo = useCallback(() => {
    setVideoEnabled(false);
  }, []);

  const audioPreview = usePreviewTrack({
    deviceId: audioDeviceId,
    enabled: audioEnabled,
    hasMediaDevices,
    kind: 'audio',
    onUnavailable: disableUnavailableAudio,
    refreshDevices,
  });
  const videoPreview = usePreviewTrack({
    deviceId: videoDeviceId,
    enabled: videoEnabled,
    hasMediaDevices,
    kind: 'video',
    onUnavailable: disableUnavailableVideo,
    refreshDevices,
  });

  useEffect(() => {
    void refreshDevices();
  }, [refreshDevices]);

  useEffect(() => {
    if (!audioPreview.stream || !audioEnabled) {
      setAudioLevel(0);
      return;
    }

    const AudioContextCtor =
      window.AudioContext ||
      (window as Window & typeof globalThis & {
        webkitAudioContext?: typeof AudioContext;
      }).webkitAudioContext;
    if (!AudioContextCtor) {
      return;
    }

    const audioContext = new AudioContextCtor();
    const analyser = audioContext.createAnalyser();
    const source = audioContext.createMediaStreamSource(audioPreview.stream);
    const data = new Uint8Array(analyser.frequencyBinCount);
    let frameId = 0;
    let smoothedLevel = 0;

    analyser.fftSize = 1024;
    source.connect(analyser);
    void audioContext.resume();

    const updateLevel = () => {
      analyser.getByteTimeDomainData(data);
      const sumOfSquares = data.reduce((sum, value) => {
        const centeredValue = (value - 128) / 128;
        return sum + centeredValue * centeredValue;
      }, 0);
      const rms = Math.sqrt(sumOfSquares / data.length);
      const nextLevel = Math.min(
        100,
        Math.max(0, Math.round(((rms - 0.005) / 0.055) * 100)),
      );

      smoothedLevel = smoothedLevel * 0.72 + nextLevel * 0.28;
      setAudioLevel(Math.round(smoothedLevel));
      frameId = window.requestAnimationFrame(updateLevel);
    };

    updateLevel();

    return () => {
      window.cancelAnimationFrame(frameId);
      source.disconnect();
      void audioContext.close();
    };
  }, [audioEnabled, audioPreview.stream]);

  return {
    audio: {
      deviceId: audioDeviceId,
      devices: audioDevices,
      enabled: audioEnabled,
      error: audioPreview.error,
      level: audioLevel,
      setDeviceId: setAudioDeviceId,
      setEnabled: setAudioEnabled,
      stream: audioPreview.stream,
    },
    hasMediaDevices,
    video: {
      deviceId: videoDeviceId,
      devices: videoDevices,
      enabled: videoEnabled,
      error: videoPreview.error,
      setDeviceId: setVideoDeviceId,
      setEnabled: setVideoEnabled,
      stream: videoPreview.stream,
    },
  };
};
