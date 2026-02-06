package me.aligator.e_chess.service

import android.content.Context
import android.net.Uri
import android.util.Log
import androidx.core.content.FileProvider
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import java.io.File
import java.io.FileOutputStream
import java.io.InterruptedIOException

/**
 * Minimal log collector that records the current process' logcat output into
 * an app-internal cache file. Intended for on-demand debugging by the user.
 */
object DebugLogManager {
    private const val LOG_TAG = "DebugLogManager"
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var collectorJob: Job? = null
    private var logcatProcess: Process? = null
    private var logFile: File? = null
    @Volatile
    private var stopping = false

    fun isRunning(): Boolean = collectorJob?.isActive == true

    /**
     * Starts capturing logcat for this process. If already running, this is a no-op.
     */
    fun start(context: Context) {
        if (isRunning()) return

        stopping = false
        val file = buildLogFile(context)
        file.parentFile?.mkdirs()
        file.writeText("Debug logging started\n")
        logFile = file

        val processBuilder = ProcessBuilder(
            "logcat",
            "--pid=${android.os.Process.myPid()}",
            "*:V"
        ).redirectErrorStream(true)

        logcatProcess = try {
            processBuilder.start()
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to start logcat process", e)
            return
        }

        collectorJob = scope.launch {
            try {
                logcatProcess?.inputStream?.use { input ->
                    FileOutputStream(file, true).buffered().use { output ->
                        output.write("Debug logging attached to logcat\n".toByteArray())
                        output.flush()
                        input.copyTo(output)
                    }
                }
            } catch (e: InterruptedIOException) {
                if (!stopping) {
                    Log.e(LOG_TAG, "Log capture interrupted unexpectedly", e)
                }
            } catch (e: Exception) {
                if (!stopping) {
                    Log.e(LOG_TAG, "Log capture failed", e)
                }
            } finally {
                runCatching {
                    logcatProcess?.inputStream?.close()
                }
                runCatching {
                    logcatProcess?.destroy()
                }
                logcatProcess = null
                if (stopping) {
                    runCatching {
                        file.appendText("\nDebug logging stopped\n")
                    }
                }
            }
        }

        // File reference already set before launching collector to avoid race.
    }

    /**
     * Stops capture and waits until the collector has fully shut down.
     */
    suspend fun stopAndAwait() {
        stopping = true
        runCatching {
            logcatProcess?.destroy()
        }
        collectorJob?.cancelAndJoin()
        collectorJob = null
        logcatProcess = null
    }

    /**
     * Returns the current log file if it exists and has data.
     */
    fun currentLogFile(): File? {
        val file = existingLogFile() ?: return null
        return file.takeIf { it.length() > 0 }
    }

    fun currentLogFileName(): String? = existingLogFile()?.name

    fun currentLogFileSizeBytes(): Long? = existingLogFile()?.length()

    suspend fun clearAllLogs(context: Context) {
        stopAndAwait()

        existingLogFile()?.let { file ->
            runCatching { file.delete() }
        }
        logFile = null

        context.cacheDir.listFiles()?.forEach { file ->
            if (file.isFile && file.name.startsWith("debug-log-")) {
                runCatching { file.delete() }
            }
        }

        val shareDir = File(context.cacheDir, "shared-logs")
        if (shareDir.exists() && shareDir.isDirectory) {
            shareDir.listFiles()?.forEach { file ->
                if (file.isFile) {
                    runCatching { file.delete() }
                }
            }
        }
    }

    /**
     * Returns a FileProvider URI for the current log file so it can be shared.
     */
    fun shareUri(context: Context): Uri? {
        val file = exportShareCopy(context) ?: return null
        return FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            file
        )
    }

    private fun buildLogFile(context: Context): File =
        File(context.cacheDir, "debug-log-${System.currentTimeMillis()}.txt")

    private fun existingLogFile(): File? =
        logFile?.takeIf { it.exists() }

    /**
     * Creates an immutable copy for sharing so the recipient does not read a
     * moving/locked file handle from the active logger file.
     */
    private fun exportShareCopy(context: Context): File? {
        val source = currentLogFile() ?: return null
        val shareDir = File(context.cacheDir, "shared-logs")
        shareDir.mkdirs()

        val target = File(
            shareDir,
            "shared-${System.currentTimeMillis()}-${source.name}"
        )
        return try {
            source.inputStream().use { input ->
                FileOutputStream(target, false).use { output ->
                    input.copyTo(output)
                }
            }
            target.takeIf { it.exists() && it.length() > 0 }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to export log for sharing", e)
            null
        }
    }
}
