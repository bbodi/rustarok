import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.await
import kotlinx.coroutines.launch
import org.khronos.webgl.ArrayBufferView
import org.khronos.webgl.Uint8Array
import org.w3c.dom.WorkerGlobalScope
import org.w3c.files.Blob
import kotlin.js.Promise

external interface IDBFactory {

}

external interface IDBDatabase {

}

data class TextureEntry(val path: String, val hash: String, val count: Int)

object IndexedDb {
    private val indexedDb: IDBFactory = js("window.indexedDB || window.mozIndexedDB || window.webkitIndexedDB || window.msIndexedDB")

    suspend fun collect_mismatched_textures(entries: Map<String, DatabaseTextureEntry>): ArrayList<String> {
        val db = open()
        val tx = db.transaction("textures", "readwrite")
        val store = tx.objectStore("textures")
        val mismatched_textures = arrayListOf<String>()
        for (entry in entries) {
            val client_entry: TextureEntry? = make_await { store.get(entry.key) }
            if (client_entry != null) {
                if (client_entry.path == "[100, 97, 116, 97, 92, 115, 112, 114, 105, 116, 101, 92, 195, 128, 195, 142, 194, 176, 194, 163, 195, 129, 194, 183, 92, 194, 184, 195, 182, 195, 133, 195, 171, 92, 194, 191, 194, 169, 92, 195, 133, 194, 169, 194, 183, 195, 167, 194, 188, 194, 188, 195, 128, 195, 140, 194, 180, 195, 181, 95, 194, 191, 194, 169]") {
                    js("debugger")
                }
                if (client_entry.count != entry.value.gl_textures.size ||
                        client_entry.hash != entry.value.hash) {
                    mismatched_textures.add(entry.key)
                }
            } else {
                mismatched_textures.add(entry.key)
            }
        }
        return mismatched_textures
    }

    private suspend fun open(): dynamic {
        val db = make_await<dynamic> {
            val req = indexedDb.asDynamic().open("rustarokDB")
            req.onupgradeneeded = { event: dynamic ->
                // Save the IDBDatabase interface
                val db: dynamic = event.target.result;

                // Create an objectStore for this database
                val objectStore = db.createObjectStore("textures", object {
                    val keyPath = "path"
                })
                val objectStore2 = db.createObjectStore("texture_data")
            }
            req.onerror = { event: dynamic ->
                console.error("DB error: " + event.target.errorCode)
            }
            req
        }
        return db
    }

    suspend fun store_texture_info(path: String, hash: String, count: Int) {
        val db = open()
        val tx = db.transaction("textures", "readwrite")
        val store = tx.objectStore("textures")
        make_await<dynamic> {
            store.put(object {
                val path = path
                val hash = hash
                val count = count
            })
        }
    }

    suspend fun store_texture(path: String, texture_index: Int, w: Int, h: Int, rawData: Uint8Array) {
        val db = open()
        val tx = db.transaction("texture_data", "readwrite")
        val store = tx.objectStore("texture_data")
        val key = "${path}_$texture_index"
        val result = make_await<dynamic> {
            store.put(object {
                val w = w
                val h = h
                val raw = rawData
            }, key)
        }
        if (result != key) {
            js("debugger")
        }
    }

    suspend fun get_texture(path: String, i: Int): Uint8Array? {
        val db = open()
        val tx = db.transaction("texture_data", "readwrite")
        val store = tx.objectStore("texture_data")
        val sh = make_await<dynamic> { store.get("${path}_$i") }
        return if (sh != null) {
//            js("debugger")
            sh.raw
        } else {
            null
        }
    }
}

suspend fun <T> make_await(block: () -> dynamic): T {
    return Promise<T> { resolve, reject ->
        val req = block()
        req.onsuccess = {
            resolve(req.result)
        }
        req.onerror = { event: dynamic ->
            reject(event)
        }
    }.catch { e ->
        console.error(e)
        throw e
        0.asDynamic()
    }.await()
}