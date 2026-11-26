import org.gradle.internal.os.OperatingSystem

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

/** Racine du workspace Cargo (le dépôt). */
val cargoRoot: File = rootDir.parentFile
val rustJniLibs = layout.buildDirectory.dir("rustJniLibs")
val uniffiOut = layout.buildDirectory.dir("generated/uniffi")
val rustProfile = providers.gradleProperty("rustProfile").getOrElse("release")

/** Signature de publication : fournie par la CI (secrets du dépôt) ou par l'environnement. */
val keystorePath: String? = System.getenv("ANDROID_KEYSTORE_PATH")
/** Numéro de build Play Store (croissant) et version affichée, injectés par la CI. */
val ciVersionCode = System.getenv("ANDROID_VERSION_CODE")?.toIntOrNull() ?: 1
val ciVersionName = System.getenv("FONDERIE_VERSION")?.trim()?.removePrefix("v")
    ?.takeIf { it.isNotEmpty() } ?: "0.16.0"

android {
    namespace = "io.github.ajustor.fonderie"
    compileSdk = 34
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "io.github.ajustor.fonderie"
        minSdk = 26
        targetSdk = 34
        versionCode = ciVersionCode
        versionName = ciVersionName
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    signingConfigs {
        if (keystorePath != null) {
            create("upload") {
                storeFile = file(keystorePath)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Sans clé (compilation locale), la release est signée avec la clé de debug.
            signingConfig = signingConfigs.findByName("upload") ?: signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir(rustJniLibs)
            java.srcDir(uniffiOut)
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation(platform("androidx.compose:compose-bom:2024.09.03"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.9.2")
}

/** Compile `libfonderie.so` pour chaque ABI avec cargo-ndk. */
val cargoNdk by tasks.registering(Exec::class) {
    group = "rust"
    workingDir = cargoRoot
    val args = mutableListOf(
        "cargo", "ndk",
        "-t", "arm64-v8a", "-t", "x86_64",
        "-P", "26",
        "-o", rustJniLibs.get().asFile.absolutePath,
        "build", "-p", "fonderie-android",
    )
    if (rustProfile == "release") args += "--release"
    commandLine(args)
    environment("ANDROID_NDK_HOME", android.ndkDirectory.absolutePath)
    inputs.dir(File(cargoRoot, "crates"))
    inputs.file(File(cargoRoot, "Cargo.lock"))
    outputs.dir(rustJniLibs)
}

/** Génère les bindings Kotlin depuis une compilation hôte (non strippée). */
val uniffiBindings by tasks.registering(Exec::class) {
    group = "rust"
    workingDir = cargoRoot
    val ext = when {
        OperatingSystem.current().isMacOsX -> "dylib"
        OperatingSystem.current().isWindows -> "dll"
        else -> "so"
    }
    val lib = if (OperatingSystem.current().isWindows) "fonderie.$ext" else "libfonderie.$ext"
    commandLine(
        "sh", "-c",
        "cargo build -p fonderie-android && cargo run -q -p uniffi-bindgen -- generate " +
            "--library target/debug/$lib --language kotlin --no-format " +
            "--out-dir '${uniffiOut.get().asFile.absolutePath}'",
    )
    inputs.dir(File(cargoRoot, "crates/android/src"))
    outputs.dir(uniffiOut)
}

tasks.named("preBuild") {
    dependsOn(cargoNdk, uniffiBindings)
}
