import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  ActivityIndicator,
  BackHandler,
  Modal,
  Platform,
  Pressable,
  SafeAreaView,
  StatusBar,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { WebView } from 'react-native-webview';
import SplashScreen from './src/SplashScreen';
import { BRAND, DEFAULT_URL, URL_PRESETS } from './src/config';

export default function App() {
  const [splashDone, setSplashDone] = useState(false);
  const [url, setUrl] = useState(DEFAULT_URL);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [draftUrl, setDraftUrl] = useState(DEFAULT_URL);
  const [canGoBack, setCanGoBack] = useState(false);
  const webRef = useRef(null);

  // Always launch on the default URL (http://187.127.137.38:7000/).
  // The ⚙ switcher still lets you change it for the current session only.

  // Android hardware back -> WebView history
  useEffect(() => {
    if (Platform.OS !== 'android') return;
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      if (canGoBack && webRef.current) {
        webRef.current.goBack();
        return true;
      }
      return false;
    });
    return () => sub.remove();
  }, [canGoBack]);

  const saveUrl = useCallback((next) => {
    let v = (next || '').trim();
    if (!/^https?:\/\//i.test(v)) v = 'http://' + v;
    setUrl(v);
    setError(null);
    setLoading(true);
    setSettingsOpen(false);
  }, []);

  const reload = useCallback(() => {
    setError(null);
    setLoading(true);
    webRef.current && webRef.current.reload();
  }, []);

  return (
    <View style={styles.root}>
      <StatusBar barStyle="light-content" backgroundColor={BRAND.slate2} />
      <SafeAreaView style={styles.safe}>
        {/* WebView */}
        {!error ? (
          <WebView
            ref={webRef}
            source={{ uri: url }}
            originWhitelist={['*']}
            onLoadStart={() => setLoading(true)}
            onLoadEnd={() => setLoading(false)}
            onNavigationStateChange={(s) => setCanGoBack(s.canGoBack)}
            onError={(e) => {
              setLoading(false);
              setError(e.nativeEvent.description || 'Failed to load');
            }}
            onHttpError={(e) => {
              setLoading(false);
              setError('HTTP ' + e.nativeEvent.statusCode);
            }}
            startInLoadingState
            mixedContentMode="always"
            domStorageEnabled
            javaScriptEnabled
            pullToRefreshEnabled
            allowsBackForwardNavigationGestures
            style={styles.web}
          />
        ) : (
          <View style={styles.errorWrap}>
            <Text style={styles.errorTitle}>Can't reach Odysseus</Text>
            <Text style={styles.errorMsg}>{error}</Text>
            <Text style={styles.errorUrl}>{url}</Text>
            <TouchableOpacity style={styles.btnPrimary} onPress={reload}>
              <Text style={styles.btnPrimaryText}>Retry</Text>
            </TouchableOpacity>
            <TouchableOpacity style={styles.btnGhost} onPress={() => setSettingsOpen(true)}>
              <Text style={styles.btnGhostText}>Change server URL</Text>
            </TouchableOpacity>
          </View>
        )}

        {/* top loading bar */}
        {loading && !error && (
          <View style={styles.loadingBar}>
            <ActivityIndicator color={BRAND.primary} />
          </View>
        )}

        {/* floating settings button */}
        <TouchableOpacity
          style={styles.fab}
          onPress={() => {
            setDraftUrl(url);
            setSettingsOpen(true);
          }}
          activeOpacity={0.8}
        >
          <Text style={styles.fabText}>⚙</Text>
        </TouchableOpacity>
      </SafeAreaView>

      {/* settings / URL switcher */}
      <Modal visible={settingsOpen} animationType="slide" transparent onRequestClose={() => setSettingsOpen(false)}>
        <Pressable style={styles.modalBackdrop} onPress={() => setSettingsOpen(false)} />
        <View style={styles.sheet}>
          <Text style={styles.sheetTitle}>Server URL</Text>
          <Text style={styles.sheetSub}>Where Odysseus is running</Text>

          {URL_PRESETS.map((p) => (
            <TouchableOpacity key={p.url} style={styles.preset} onPress={() => setDraftUrl(p.url)}>
              <Text style={styles.presetLabel}>{p.label}</Text>
              <Text style={styles.presetUrl}>{p.url}</Text>
            </TouchableOpacity>
          ))}

          <TextInput
            value={draftUrl}
            onChangeText={setDraftUrl}
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
            placeholder="http://host:7000/"
            placeholderTextColor="#94a3b8"
            style={styles.input}
          />

          <TouchableOpacity style={styles.btnPrimary} onPress={() => saveUrl(draftUrl)}>
            <Text style={styles.btnPrimaryText}>Connect</Text>
          </TouchableOpacity>
          <TouchableOpacity style={styles.btnGhost} onPress={() => setSettingsOpen(false)}>
            <Text style={styles.btnGhostText}>Cancel</Text>
          </TouchableOpacity>
        </View>
      </Modal>

      {/* animated splash overlays everything until done */}
      {!splashDone && <SplashScreen onFinish={() => setSplashDone(true)} />}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: BRAND.slate2 },
  safe: { flex: 1, backgroundColor: '#ffffff' },
  web: { flex: 1 },
  loadingBar: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    paddingVertical: 8,
    alignItems: 'center',
  },
  fab: {
    position: 'absolute',
    right: 16,
    bottom: 24,
    width: 48,
    height: 48,
    borderRadius: 24,
    backgroundColor: BRAND.primary,
    alignItems: 'center',
    justifyContent: 'center',
    shadowColor: '#000',
    shadowOpacity: 0.3,
    shadowRadius: 8,
    shadowOffset: { width: 0, height: 4 },
    elevation: 6,
  },
  fabText: { color: '#fff', fontSize: 22, lineHeight: 26 },
  errorWrap: { flex: 1, alignItems: 'center', justifyContent: 'center', padding: 28 },
  errorTitle: { fontSize: 20, fontWeight: '700', color: BRAND.slate, marginBottom: 8 },
  errorMsg: { color: '#64748b', textAlign: 'center', marginBottom: 4 },
  errorUrl: { color: BRAND.primary, marginBottom: 24, fontSize: 12 },
  modalBackdrop: { ...StyleSheet.absoluteFillObject, backgroundColor: 'rgba(15,23,42,0.55)' },
  sheet: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: '#fff',
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    padding: 22,
    paddingBottom: 36,
  },
  sheetTitle: { fontSize: 18, fontWeight: '700', color: BRAND.slate },
  sheetSub: { color: '#64748b', marginBottom: 16 },
  preset: {
    borderWidth: 1,
    borderColor: '#e2e8f0',
    borderRadius: 12,
    padding: 12,
    marginBottom: 10,
  },
  presetLabel: { fontWeight: '600', color: BRAND.slate },
  presetUrl: { color: '#64748b', fontSize: 12, marginTop: 2 },
  input: {
    borderWidth: 1,
    borderColor: '#cbd5e1',
    borderRadius: 12,
    padding: 12,
    marginTop: 6,
    marginBottom: 16,
    color: BRAND.slate,
  },
  btnPrimary: {
    backgroundColor: BRAND.primary,
    borderRadius: 12,
    paddingVertical: 14,
    alignItems: 'center',
  },
  btnPrimaryText: { color: '#fff', fontWeight: '700', fontSize: 16 },
  btnGhost: { paddingVertical: 12, alignItems: 'center' },
  btnGhostText: { color: '#64748b', fontWeight: '600' },
});
