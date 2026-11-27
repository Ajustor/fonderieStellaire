# JNA charge la bibliothèque Rust et retrouve ses classes par réflexion
# (structures, callbacks, interfaces de bibliothèque) : rien n'est renommé.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**

# Bindings UniFFI générés : appelés par JNA, champs lus par réflexion.
-keep class uniffi.** { *; }
