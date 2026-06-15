import React, { useEffect, useRef } from 'react';
import { Animated, Pressable, StyleSheet } from 'react-native';
import { useVideoPlayer, VideoView } from 'expo-video';
import { BRAND } from './config';

const source = require('../assets/branding/splash.mp4');

// Video splash: plays oms_odysseus.mp4 full-screen, then fades into the app.
// Tap to skip; safety timeout in case the end event never fires.
export default function SplashScreen({ onFinish }) {
  const fade = useRef(new Animated.Value(1)).current;
  const done = useRef(false);

  const player = useVideoPlayer(source, (p) => {
    p.loop = false;
    p.muted = false; // branded intro audio; set true to silence
    p.play();
  });

  const finish = () => {
    if (done.current) return;
    done.current = true;
    Animated.timing(fade, {
      toValue: 0,
      duration: 400,
      useNativeDriver: true,
    }).start(() => onFinish && onFinish());
  };

  useEffect(() => {
    const sub = player.addListener('playToEnd', finish);
    const safety = setTimeout(finish, 12000);
    return () => {
      sub.remove();
      clearTimeout(safety);
    };
  }, [player]);

  return (
    <Animated.View style={[styles.root, { opacity: fade }]}>
      <Pressable style={StyleSheet.absoluteFill} onPress={finish}>
        <VideoView
          player={player}
          style={StyleSheet.absoluteFill}
          contentFit="cover"
          nativeControls={false}
          allowsFullscreen={false}
          allowsPictureInPicture={false}
        />
      </Pressable>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  root: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: BRAND.slate2,
    zIndex: 10,
  },
});
