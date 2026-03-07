package io.visio.mobile

import android.bluetooth.BluetoothDevice
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.util.Log
import uniffi.visio.NetworkType

class ContextDetector(private val context: Context) {

    private val connectivityManager =
        context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    private val sensorManager =
        context.getSystemService(Context.SENSOR_SERVICE) as SensorManager
    private val audioManager =
        context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var accelerometerListener: SensorEventListener? = null
    private var bluetoothReceiver: BroadcastReceiver? = null

    private var lastAccelTimestamp = 0L
    private var motionCount = 0
    private var lastReportedMotion = false
    private var motionSinceMs = 0L          // when continuous motion started
    private var stillSinceMs = 0L           // when motion last stopped
    private val MOTION_THRESHOLD = 1.2f
    private val MOTION_WINDOW_MS = 2000L
    private val MOTION_COUNT_THRESHOLD = 6
    private val MOTION_CONFIRM_MS = 3000L   // must move 3s before switching to pedestrian
    private val MOTION_COOLDOWN_MS = 10000L // stay pedestrian 10s after motion stops

    companion object {
        private const val TAG = "ContextDetector"
    }

    fun start() {
        Log.i(TAG, "Starting context detection (network, motion, bluetooth)")
        startNetworkMonitoring()
        startMotionDetection()
        startBluetoothMonitoring()
        reportCurrentNetworkType()
        reportBluetoothCarKit()
    }

    fun stop() {
        networkCallback?.let { connectivityManager.unregisterNetworkCallback(it) }
        accelerometerListener?.let { sensorManager.unregisterListener(it) }
        bluetoothReceiver?.let {
            try { context.unregisterReceiver(it) } catch (_: Exception) {}
        }
        networkCallback = null
        accelerometerListener = null
        bluetoothReceiver = null
    }

    private fun startNetworkMonitoring() {
        val request = NetworkRequest.Builder().build()
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) {
                val type = when {
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> NetworkType.WIFI
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> NetworkType.CELLULAR
                    else -> NetworkType.UNKNOWN
                }
                Log.d(TAG, "Network type: $type")
                try { VisioManager.client.reportNetworkType(type) } catch (_: Exception) {}
            }
            override fun onLost(network: Network) {
                Log.d(TAG, "Network lost")
                try { VisioManager.client.reportNetworkType(NetworkType.UNKNOWN) } catch (_: Exception) {}
            }
        }
        connectivityManager.registerNetworkCallback(request, callback)
        networkCallback = callback
    }

    private fun reportCurrentNetworkType() {
        val caps = connectivityManager.getNetworkCapabilities(connectivityManager.activeNetwork)
        val type = when {
            caps == null -> NetworkType.UNKNOWN
            caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> NetworkType.WIFI
            caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> NetworkType.CELLULAR
            else -> NetworkType.UNKNOWN
        }
        try { VisioManager.client.reportNetworkType(type) } catch (_: Exception) {}
    }

    private var sensorEventCount = 0L

    private fun startMotionDetection() {
        val accelerometer = sensorManager.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
        if (accelerometer == null) {
            Log.w(TAG, "No accelerometer sensor available")
            return
        }
        Log.d(TAG, "Accelerometer found: ${accelerometer.name}")
        val listener = object : SensorEventListener {
            override fun onSensorChanged(event: SensorEvent) {
                sensorEventCount++
                val x = event.values[0]
                val y = event.values[1]
                val z = event.values[2]
                val magnitude = kotlin.math.sqrt((x * x + y * y + z * z).toDouble()).toFloat()
                val deviation = kotlin.math.abs(magnitude - SensorManager.GRAVITY_EARTH)
                val now = System.currentTimeMillis()
                if (now - lastAccelTimestamp > MOTION_WINDOW_MS) {
                    motionCount = 0
                    lastAccelTimestamp = now
                }

                if (deviation > MOTION_THRESHOLD) {
                    motionCount++
                }

                val rawMoving = motionCount > MOTION_COUNT_THRESHOLD

                // Debounce: confirm motion for MOTION_CONFIRM_MS before switching ON,
                // and wait MOTION_COOLDOWN_MS after stopping before switching OFF.
                if (rawMoving) {
                    stillSinceMs = 0L
                    if (motionSinceMs == 0L) motionSinceMs = now
                } else {
                    if (motionSinceMs != 0L) {
                        motionSinceMs = 0L
                        if (stillSinceMs == 0L) stillSinceMs = now
                    }
                }

                val debouncedMoving = when {
                    // Currently reported as moving: stay moving until cooldown expires
                    lastReportedMotion -> rawMoving || (stillSinceMs != 0L && now - stillSinceMs < MOTION_COOLDOWN_MS)
                    // Currently still: need sustained motion to switch
                    else -> motionSinceMs != 0L && now - motionSinceMs > MOTION_CONFIRM_MS
                }

                if (sensorEventCount % 200 == 0L) {
                    Log.d(TAG, "Accel #$sensorEventCount: dev=${"%.2f".format(deviation)} raw=$rawMoving debounced=$debouncedMoving reported=$lastReportedMotion")
                }

                if (debouncedMoving != lastReportedMotion) {
                    lastReportedMotion = debouncedMoving
                    Log.d(TAG, "Motion state changed: moving=$debouncedMoving")
                    try { VisioManager.client.reportMotionDetected(debouncedMoving) } catch (_: Exception) {}
                }
            }

            override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {}
        }
        sensorManager.registerListener(listener, accelerometer, SensorManager.SENSOR_DELAY_NORMAL)
        accelerometerListener = listener
    }

    private fun startBluetoothMonitoring() {
        val filter = IntentFilter().apply {
            addAction(BluetoothDevice.ACTION_ACL_CONNECTED)
            addAction(BluetoothDevice.ACTION_ACL_DISCONNECTED)
        }
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context, intent: Intent) {
                reportBluetoothCarKit()
            }
        }
        context.registerReceiver(receiver, filter)
        bluetoothReceiver = receiver
    }

    private fun reportBluetoothCarKit() {
        val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
        val hasCarKit = devices.any {
            it.type == AudioDeviceInfo.TYPE_BLUETOOTH_A2DP ||
            it.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO
        }
        try { VisioManager.client.reportBluetoothCarKit(hasCarKit) } catch (_: Exception) {}
    }
}
