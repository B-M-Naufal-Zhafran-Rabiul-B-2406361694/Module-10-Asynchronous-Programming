# Module 10 - Asynchronous Programming

## Experiment 1.2: Understanding How It Works

### Hasil Eksekusi

![Capture output program](capture.png)

```
Naufal's Komputer: hey hey!
Naufal's Komputer: howdy!
Naufal's Komputer: done!
```

### Penjelasan

`println!("Naufal's Komputer: hey hey!")` diletakkan **setelah** `spawner.spawn(...)` tetapi **sebelum** `drop(spawner)` dan `executor.run()`. Namun, teks tersebut justru muncul **paling pertama**, sebelum "howdy!" dan "done!".

Hal ini terjadi karena `spawner.spawn(...)` **tidak langsung menjalankan** async block. Fungsi ini hanya memasukkan future ke dalam sebuah channel (antrian). Async block masih dalam kondisi diam — belum berjalan sama sekali.

Main thread kemudian melanjutkan eksekusi secara sinkron dan menemui `println!("hey hey!")`, sehingga langsung dicetak ke terminal.

Barulah setelah `drop(spawner)` menutup channel, `executor.run()` mulai melakukan polling terhadap future yang ada di antrian. Pada titik ini async block berjalan: mencetak "howdy!", lalu berhenti sejenak selama 2 detik menunggu `TimerFuture`, dan akhirnya mencetak "done!" setelah thread timer membangunkan waker.

Singkatnya: **spawn hanya mengantrekan future, bukan menjalankannya.** Executor baru mengeksekusi future ketika `executor.run()` dipanggil. Itulah mengapa kode sinkron yang ditempatkan antara `spawn` dan `run` selalu dieksekusi lebih dulu sebelum pekerjaan async apapun dimulai.

---

## Experiment 1.3: Multiple Spawn and Removing Drop

### Hasil Eksekusi — Multiple Spawn (dengan `drop`)

```
Naufal's Komputer: hey hey!
Naufal's Komputer: howdy!
Naufal's Komputer: howdy2!
Naufal's Komputer: howdy3!
Naufal's Komputer: done3!
Naufal's Komputer: done!
Naufal's Komputer: done2!
```

### Hasil Eksekusi — Tanpa `drop(spawner)`

```
Naufal's Komputer: hey hey!
Naufal's Komputer: howdy!
Naufal's Komputer: howdy2!
Naufal's Komputer: howdy3!
Naufal's Komputer: done3!
Naufal's Komputer: done2!
Naufal's Komputer: done!
[program tidak berhenti / hang]
```

### Penjelasan

**Multiple Spawn**

Ketika tiga `spawner.spawn(...)` dipanggil, ketiganya memasukkan future ke dalam antrian channel tanpa langsung dieksekusi. Saat `executor.run()` mulai bekerja, executor mem-poll task pertama hingga task tersebut mengembalikan `Poll::Pending` (ketika menunggu timer). Lalu executor beralih ke task berikutnya, dan seterusnya. Itulah mengapa urutan "howdy!", "howdy2!", "howdy3!" muncul berurutan — ketiga task dimulai sebelum satupun selesai.

Urutan "done!" bisa tidak berurutan (done3 muncul lebih dulu) karena ketiga timer thread berjalan secara konkuren. Thread mana yang selesai lebih dulu akan me-wake task-nya lebih dulu, sehingga urutannya bergantung pada scheduling sistem operasi dan tidak deterministik.

**Menghapus `drop(spawner)`**

Ketika `drop(spawner)` dihapus (di-comment), channel **tidak pernah ditutup** karena `Spawner` masih memegang satu sender yang aktif. `executor.run()` menggunakan `while let Ok(task) = self.ready_queue.recv()` — fungsi `recv()` akan **memblokir selamanya** menunggu task baru yang tidak pernah datang setelah semua task selesai. Akibatnya program **hang** dan tidak pernah terminate meskipun semua output sudah tercetak.

`drop(spawner)` berfungsi sebagai sinyal kepada executor bahwa tidak akan ada task baru lagi, sehingga `recv()` mengembalikan `Err` dan loop berhenti dengan bersih.
