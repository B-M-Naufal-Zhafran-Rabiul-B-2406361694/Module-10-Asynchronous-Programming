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
