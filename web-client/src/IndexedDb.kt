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
        make_await<dynamic> {
            store.put(object {
                val w = w
                val h = h
                val raw = rawData
            }, "${path}_$texture_index")
        }
    }

    suspend fun get_texture(path: String, i: Int): Uint8Array {
        val db = open()
        val tx = db.transaction("texture_data", "readwrite")
        val store = tx.objectStore("texture_data")
        return make_await<dynamic> { store.get("${path}_$i") }.raw
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
    }.await()
}