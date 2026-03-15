(function () {
  "use strict";

  // __VOID_SESSION_SEED is injected by Rust as a hex string before this script.
  var SEED = typeof __VOID_SESSION_SEED === "string" ? __VOID_SESSION_SEED : "";

  // ---------------------------------------------------------------------------
  // 1. Utilities: hashing, PRNG, anti-detection helpers
  // ---------------------------------------------------------------------------

  // Simple string hash (FNV-1a 32-bit)
  function fnv1a(str) {
    var hash = 0x811c9dc5;
    for (var i = 0; i < str.length; i++) {
      hash ^= str.charCodeAt(i);
      hash = Math.imul(hash, 0x01000193);
    }
    return hash >>> 0;
  }

  // Mulberry32 seeded PRNG — returns a function that yields [0, 1) floats.
  function mulberry32(seed) {
    var s = seed | 0;
    return function () {
      s = (s + 0x6d2b79f5) | 0;
      var t = Math.imul(s ^ (s >>> 15), 1 | s);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 0x100000000;
    };
  }

  // Per-origin deterministic PRNG
  function originPrng() {
    var origin = "";
    try {
      origin = window.location.origin;
    } catch (_) {
      // cross-origin iframes may throw
    }
    return mulberry32(fnv1a(SEED + origin));
  }

  // --- Anti-detection: make overridden functions appear native ----------------

  var nativeMap = new WeakMap();
  var origToString = Function.prototype.toString;

  // Patch Function.prototype.toString once so that all overridden functions
  // return "function <name>() { [native code] }".
  try {
    Object.defineProperty(Function.prototype, "toString", {
      value: function toString() {
        if (nativeMap.has(this)) {
          return "function " + nativeMap.get(this) + "() { [native code] }";
        }
        return origToString.call(this);
      },
      writable: true,
      configurable: true,
    });
    nativeMap.set(Function.prototype.toString, "toString");
  } catch (_) {
    // If we cannot patch toString, continue anyway.
  }

  // Override a method on a prototype and register it as native-looking.
  function overrideMethod(obj, prop, fn) {
    var orig = obj[prop];
    nativeMap.set(fn, prop);
    try {
      Object.defineProperty(obj, prop, {
        value: fn,
        writable: false,
        configurable: false,
        enumerable: true,
      });
    } catch (_) {
      // Some properties cannot be redefined; fall back to assignment.
      try {
        obj[prop] = fn;
      } catch (_2) {
        // Give up silently.
      }
    }
    return orig;
  }

  // Override a getter on an object.
  function overrideGetter(obj, prop, getter) {
    nativeMap.set(getter, "get " + prop);
    try {
      Object.defineProperty(obj, prop, {
        get: getter,
        set: undefined,
        configurable: false,
        enumerable: true,
      });
    } catch (_) {
      // Silently fail if the property is locked.
    }
  }

  // ---------------------------------------------------------------------------
  // 2. Canvas fingerprinting protection
  // ---------------------------------------------------------------------------

  (function () {
    var rng = originPrng();

    // Add subtle noise to ImageData pixels.
    function addCanvasNoise(imageData) {
      var d = imageData.data;
      var len = d.length;
      // Noise ~2% of pixels, channel 0 (R) only, ±5
      for (var i = 0; i < len; i += 4) {
        if (rng() < 0.02) {
          var noise = Math.floor(rng() * 11) - 5; // -5 to +5
          var val = d[i] + noise;
          d[i] = val < 0 ? 0 : val > 255 ? 255 : val;
        }
      }
      return imageData;
    }

    // getImageData
    if (
      typeof CanvasRenderingContext2D !== "undefined" &&
      CanvasRenderingContext2D.prototype.getImageData
    ) {
      var origGetImageData =
        CanvasRenderingContext2D.prototype.getImageData;
      overrideMethod(
        CanvasRenderingContext2D.prototype,
        "getImageData",
        function getImageData(sx, sy, sw, sh) {
          var data = origGetImageData.call(this, sx, sy, sw, sh);
          return addCanvasNoise(data);
        }
      );
    }

    // toDataURL
    if (
      typeof HTMLCanvasElement !== "undefined" &&
      HTMLCanvasElement.prototype.toDataURL
    ) {
      var origToDataURL = HTMLCanvasElement.prototype.toDataURL;
      overrideMethod(
        HTMLCanvasElement.prototype,
        "toDataURL",
        function toDataURL() {
          // Inject a subtle noise pixel before exporting.
          try {
            var ctx = this.getContext("2d");
            if (ctx) {
              var r = Math.floor(rng() * 256);
              var g = Math.floor(rng() * 256);
              var px = ctx.getImageData(0, 0, 1, 1);
              // Tweak one subpixel in top-left corner
              px.data[0] = (px.data[0] + r) & 0xff;
              px.data[1] = (px.data[1] + g) & 0xff;
              // Use the original putImageData to avoid recursion
              ctx.putImageData(px, 0, 0);
            }
          } catch (_) {
            // Canvas may be tainted — ignore.
          }
          return origToDataURL.apply(this, arguments);
        }
      );
    }

    // toBlob
    if (
      typeof HTMLCanvasElement !== "undefined" &&
      HTMLCanvasElement.prototype.toBlob
    ) {
      var origToBlob = HTMLCanvasElement.prototype.toBlob;
      overrideMethod(
        HTMLCanvasElement.prototype,
        "toBlob",
        function toBlob(callback) {
          try {
            var ctx = this.getContext("2d");
            if (ctx) {
              var r = Math.floor(rng() * 256);
              var px = ctx.getImageData(0, 0, 1, 1);
              px.data[0] = (px.data[0] + r) & 0xff;
              ctx.putImageData(px, 0, 0);
            }
          } catch (_) {
            // Canvas may be tainted.
          }
          return origToBlob.apply(this, arguments);
        }
      );
    }
  })();

  // ---------------------------------------------------------------------------
  // 3. WebGL fingerprinting protection
  // ---------------------------------------------------------------------------

  (function () {
    var SPOOFED_VENDOR = "Google Inc. (Intel)";
    var SPOOFED_RENDERER =
      "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0)";

    function patchGetParameter(proto) {
      if (!proto || !proto.getParameter) return;
      var origGetParam = proto.getParameter;
      overrideMethod(proto, "getParameter", function getParameter(pname) {
        // WEBGL_debug_renderer_info extension constants
        // UNMASKED_VENDOR_WEBGL   = 0x9245
        // UNMASKED_RENDERER_WEBGL = 0x9246
        if (pname === 0x9245) return SPOOFED_VENDOR;
        if (pname === 0x9246) return SPOOFED_RENDERER;
        return origGetParam.call(this, pname);
      });
    }

    function patchGetExtension(proto) {
      if (!proto || !proto.getExtension) return;
      var origGetExt = proto.getExtension;
      overrideMethod(proto, "getExtension", function getExtension(name) {
        var ext = origGetExt.call(this, name);
        if (name === "WEBGL_debug_renderer_info" && ext) {
          // Return an object with the correct constants so that code using
          // ext.UNMASKED_VENDOR_WEBGL as a pname still works, but our
          // patched getParameter will return the spoofed values.
          return {
            UNMASKED_VENDOR_WEBGL: 0x9245,
            UNMASKED_RENDERER_WEBGL: 0x9246,
          };
        }
        return ext;
      });
    }

    if (typeof WebGLRenderingContext !== "undefined") {
      patchGetParameter(WebGLRenderingContext.prototype);
      patchGetExtension(WebGLRenderingContext.prototype);
    }
    if (typeof WebGL2RenderingContext !== "undefined") {
      patchGetParameter(WebGL2RenderingContext.prototype);
      patchGetExtension(WebGL2RenderingContext.prototype);
    }
  })();

  // ---------------------------------------------------------------------------
  // 4. AudioContext fingerprinting protection
  // ---------------------------------------------------------------------------

  (function () {
    if (typeof OfflineAudioContext === "undefined") return;
    var rng = originPrng();
    var origStartRendering =
      OfflineAudioContext.prototype.startRendering;

    overrideMethod(
      OfflineAudioContext.prototype,
      "startRendering",
      function startRendering() {
        var localRng = rng; // capture for the then-callback
        return origStartRendering.call(this).then(function (buffer) {
          try {
            var channel = buffer.getChannelData(0);
            var len = Math.min(channel.length, 100);
            for (var i = 0; i < len; i++) {
              channel[i] += (localRng() - 0.5) * 0.0002; // ±0.0001
            }
          } catch (_) {
            // Buffer may be detached or read-only.
          }
          return buffer;
        });
      }
    );
  })();

  // ---------------------------------------------------------------------------
  // 5. Navigator spoofing
  // ---------------------------------------------------------------------------

  (function () {
    var nav = typeof navigator !== "undefined" ? navigator : null;
    if (!nav) return;

    overrideGetter(nav, "hardwareConcurrency", function () {
      return 4;
    });
    overrideGetter(nav, "deviceMemory", function () {
      return 8;
    });
    overrideGetter(nav, "platform", function () {
      return "Win32";
    });

    var frozenLangs = Object.freeze(["en-US", "en"]);
    overrideGetter(nav, "languages", function () {
      return frozenLangs;
    });
    overrideGetter(nav, "language", function () {
      return "en-US";
    });

    // getBattery — return rejected promise
    overrideMethod(nav, "getBattery", function getBattery() {
      return Promise.reject(
        new DOMException("Battery API disabled", "NotAllowedError")
      );
    });

    // getGamepads — return empty array
    overrideMethod(nav, "getGamepads", function getGamepads() {
      return [];
    });

    // Block hardware APIs
    var blockedApis = ["bluetooth", "usb", "serial", "hid"];
    for (var i = 0; i < blockedApis.length; i++) {
      (function (api) {
        overrideGetter(nav, api, function () {
          return undefined;
        });
      })(blockedApis[i]);
    }
  })();

  // ---------------------------------------------------------------------------
  // 6. Timing protection — reduce performance.now() resolution to 100μs
  // ---------------------------------------------------------------------------

  (function () {
    if (typeof performance === "undefined" || !performance.now) return;
    var origNow = performance.now.bind(performance);
    var rng = originPrng();

    overrideMethod(performance, "now", function now() {
      var t = origNow();
      // Round to 0.1ms (100 microseconds)
      t = Math.round(t * 10) / 10;
      // Add tiny deterministic jitter (0 to 0.05ms)
      t += rng() * 0.05;
      return t;
    });
  })();

  // ---------------------------------------------------------------------------
  // 7. Screen spoofing
  // ---------------------------------------------------------------------------

  (function () {
    if (typeof screen === "undefined") return;

    overrideGetter(screen, "width", function () {
      return 1920;
    });
    overrideGetter(screen, "height", function () {
      return 1080;
    });
    overrideGetter(screen, "availWidth", function () {
      return 1920;
    });
    overrideGetter(screen, "availHeight", function () {
      return 1040;
    });
    overrideGetter(screen, "colorDepth", function () {
      return 24;
    });
    overrideGetter(screen, "pixelDepth", function () {
      return 24;
    });

    if (typeof window !== "undefined") {
      overrideGetter(window, "devicePixelRatio", function () {
        return 1;
      });
      overrideGetter(window, "outerWidth", function () {
        return 1920;
      });
      overrideGetter(window, "outerHeight", function () {
        return 1080;
      });
    }
  })();

  // ---------------------------------------------------------------------------
  // 8. WebRTC leak prevention
  // ---------------------------------------------------------------------------

  (function () {
    var RTC =
      typeof RTCPeerConnection !== "undefined"
        ? RTCPeerConnection
        : typeof webkitRTCPeerConnection !== "undefined"
          ? webkitRTCPeerConnection
          : null;
    if (!RTC) return;

    var OrigRTC = RTC;

    function PatchedRTCPeerConnection(config, constraints) {
      // Force empty iceServers to prevent STUN/TURN IP leak
      config = config || {};
      config.iceServers = [];

      var pc = new OrigRTC(config, constraints);

      // Wrap onicecandidate to strip non-.local candidates
      var origDescriptor = Object.getOwnPropertyDescriptor(
        OrigRTC.prototype,
        "onicecandidate"
      );

      if (origDescriptor && origDescriptor.set) {
        var origSet = origDescriptor.set;
        Object.defineProperty(pc, "onicecandidate", {
          get: function () {
            return this._voidIceCb || null;
          },
          set: function (cb) {
            this._voidIceCb = cb;
            origSet.call(this, function (event) {
              if (
                event &&
                event.candidate &&
                event.candidate.candidate
              ) {
                // Allow only .local mDNS candidates
                if (
                  event.candidate.candidate.indexOf(".local") === -1
                ) {
                  return; // suppress non-local candidate
                }
              }
              if (typeof cb === "function") {
                cb(event);
              }
            });
          },
          configurable: true,
          enumerable: true,
        });
      }

      return pc;
    }

    // Copy static properties and prototype
    PatchedRTCPeerConnection.prototype = OrigRTC.prototype;
    PatchedRTCPeerConnection.generateCertificate =
      OrigRTC.generateCertificate;
    nativeMap.set(PatchedRTCPeerConnection, "RTCPeerConnection");

    try {
      if (typeof window !== "undefined") {
        Object.defineProperty(window, "RTCPeerConnection", {
          value: PatchedRTCPeerConnection,
          writable: false,
          configurable: false,
          enumerable: true,
        });
        // Also patch the webkit prefixed version
        if (typeof webkitRTCPeerConnection !== "undefined") {
          Object.defineProperty(window, "webkitRTCPeerConnection", {
            value: PatchedRTCPeerConnection,
            writable: false,
            configurable: false,
            enumerable: true,
          });
        }
      }
    } catch (_) {
      // Cannot override — silently continue.
    }
  })();
})();
