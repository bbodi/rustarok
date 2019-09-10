package rustarok

import org.khronos.webgl.Float32Array
import org.khronos.webgl.get
import org.khronos.webgl.set
import kotlin.js.Math


/**
 * 4x4 Matrix<br>Format: column-major, when typed out it looks like row-major<br>The matrices are being post multiplied.
 * @module mat4
 */

class Matrix {
    val buffer: Float32Array = Float32Array(16)
    init {
        buffer[0] = 1f
        buffer[5] = 1f
        buffer[10] = 1f
        buffer[15] = 1f
    }

    operator fun set(i: Int, value: Float) {
        buffer[i] = value
    }

    operator fun get(i: Int): Float {
        return buffer[i]
    }

    fun clone(): Matrix {
        val out = Matrix()
        out[0] = this[0]
        out[1] = this[1]
        out[2] = this[2]
        out[3] = this[3]
        out[4] = this[4]
        out[5] = this[5]
        out[6] = this[6]
        out[7] = this[7]
        out[8] = this[8]
        out[9] = this[9]
        out[10] = this[10]
        out[11] = this[11]
        out[12] = this[12]
        out[13] = this[13]
        out[14] = this[14]
        out[15] = this[15]
        return out
    }

    fun set_translation(x: Float, y: Float, z: Float) {
        this[12] = x
        this[13] = y
        this[14] = z
    }

    fun transponse_mut() {
        val a01 = this[1]
        val a02 = this[2]
        val a03 = this[3]
        val a12 = this[6]
        val a13 = this[7]
        val a23 = this[11]

        this[1] = this[4]
        this[2] = this[8]
        this[3] = this[12]
        this[4] = a01
        this[6] = this[9]
        this[7] = this[13]
        this[8] = a02
        this[9] = a12
        this[11] = this[14]
        this[12] = a03
        this[13] = a13
        this[14] = a23
    }

    fun transpose(): Matrix {
        val out = Matrix()
        out[0] = this[0]
        out[1] = this[4]
        out[2] = this[8]
        out[3] = this[12]
        out[4] = this[1]
        out[5] = this[5]
        out[6] = this[9]
        out[7] = this[13]
        out[8] = this[2]
        out[9] = this[6]
        out[10] = this[10]
        out[11] = this[14]
        out[12] = this[3]
        out[13] = this[7]
        out[14] = this[11]
        out[15] = this[15]
        return out
    }

    fun rotate_around_y_mut(rad: Float) {
        val s = kotlin.math.sin(rad)
        val c = kotlin.math.cos(rad)
        val a00 = this[0]
        val a01 = this[1]
        val a02 = this[2]
        val a03 = this[3]
        val a20 = this[8]
        val a21 = this[9]
        val a22 = this[10]
        val a23 = this[11]

        // Perform axis-specific matrix multiplication
        this[0] = a00 * c - a20 * s
        this[1] = a01 * c - a21 * s
        this[2] = a02 * c - a22 * s
        this[3] = a03 * c - a23 * s
        this[8] = a00 * s + a20 * c
        this[9] = a01 * s + a21 * c
        this[10] = a02 * s + a22 * c
        this[11] = a03 * s + a23 * c
    }

    fun rotate_around_z_mut(rad: Float) {
        val s = kotlin.math.sin(rad)
        val c = kotlin.math.cos(rad)
        val a00 = this[0]
        val a01 = this[1]
        val a02 = this[2]
        val a03 = this[3]
        val a10 = this[4]
        val a11 = this[5]
        val a12 = this[6]
        val a13 = this[7]

        // Perform axis-specific matrix multiplication
        this[0] = a00 * c + a10 * s
        this[1] = a01 * c + a11 * s
        this[2] = a02 * c + a12 * s
        this[3] = a03 * c + a13 * s
        this[4] = a10 * c - a00 * s
        this[5] = a11 * c - a01 * s
        this[6] = a12 * c - a02 * s
        this[7] = a13 * c - a03 * s
    }


    fun rotate_mut(rad: Float, axis: Array<Float>) {
        var x = axis[0]
        var y = axis[1]
        var z = axis[2]
        val len: Float = 1f / (js("Math.hypot(x, y, z)") as Float)
        x *= len
        y *= len
        z *= len

        val s = kotlin.math.sin(rad)
        val c = kotlin.math.cos(rad)
        val t = 1 - c

        val a00 = this[0]
        val a01 = this[1]
        val a02 = this[2]
        val a03 = this[3]
        val a10 = this[4]
        val a11 = this[5]
        val a12 = this[6]
        val a13 = this[7]
        val a20 = this[8]
        val a21 = this[9]
        val a22 = this[10]
        val a23 = this[11]

        // Construct the elements of the rotation matrix
        val b00 = x * x * t + c
        val b01 = y * x * t + z * s
        val b02 = z * x * t - y * s
        val b10 = x * y * t - z * s
        val b11 = y * y * t + c
        val b12 = z * y * t + x * s
        val b20 = x * z * t + y * s
        val b21 = y * z * t - x * s
        val b22 = z * z * t + c


        // Perform rotation-specific matrix multiplication
        this[0] = a00 * b00 + a10 * b01 + a20 * b02
        this[1] = a01 * b00 + a11 * b01 + a21 * b02
        this[2] = a02 * b00 + a12 * b01 + a22 * b02
        this[3] = a03 * b00 + a13 * b01 + a23 * b02
        this[4] = a00 * b10 + a10 * b11 + a20 * b12
        this[5] = a01 * b10 + a11 * b11 + a21 * b12
        this[6] = a02 * b10 + a12 * b11 + a22 * b12
        this[7] = a03 * b10 + a13 * b11 + a23 * b12
        this[8] = a00 * b20 + a10 * b21 + a20 * b22
        this[9] = a01 * b20 + a11 * b21 + a21 * b22
        this[10] = a02 * b20 + a12 * b21 + a22 * b22
        this[11] = a03 * b20 + a13 * b21 + a23 * b22
    }
}