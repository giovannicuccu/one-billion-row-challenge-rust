
### 18/02/2024
sviluppato ILP con parallelismo 3 e da 16.9 sono passato a 12.6 secondi un guadagno del 20%
con ILP4 4 non ci sono cambiamenti significativi
con ILP2 perdo circa 0.6 secondi ILP3 è il numero ottimo

sviluppata la versione multithread, ma i benefici rispetto alla parte sequenziale sono veramente pochi rispetto al numero dei 
core attivi, capire perchè

la versione usata sotto wsl è più veloce di quella nativa window, ma di parecchio, su linux vedo anche tempi sotto il secondo, 
sotto windows sono a circa 2,7 secondi

ho confrontato i tempi sotto WSL della versione da cui sono partito ed è più veloce di 0.3 secondi circa, ma la mia soluzione non usa unsafe per cui
la ritengo competitiva con unsafe al posto di try_into guadagno 0.1 sec circa



### 17/02/2024
introdotto mmap che ha cambiato il comportamento: prima i tempi erano sempre stabili, adesso la prima esecuzione è lenta
le successive sono molto più veloci  la prima era di 23 secondi, le altre di circa 16, quindi una volta che i dati sono stati
bufferizzati ho risparmiato circa 6 secondi

TODO: provare a restituire il un result dalla mappa e modificare quello



### 12/02/2024
ho provato ad ottimizzare la try into sostituendola con una copy from slice, i risultati sono trascurabili
tengo la versione con try_into
questo post dice qualcosa di più https://lukas-prokop.at/articles/2021-11-10-array-slices-performance

perf stat sotto wsl2 non riporta tutte le istruzioni vedi questo https://github.com/microsoft/WSL/issues/8480




### 11/02/2024
la versione con hashmap custom ci mette 38 secondi, 4 secondi in meno di quella con mappa standard
la versione che ottimizza per i primi 16 byte mandando giù i128 ci mette 31 secondi, 7 in meno della versione precedente
la funzione hash occupa ancora parecchio tempo
se faccio hash del raw data i tempi scendono ancora a circa 24 secondi

### 10/02/2024

**Dopo aver reinstallato perf con le lib sotto il flamegraph sembra funzionare molto meglio**

la maggior parte del tempo è spesa nella hashmap ne devo implementare una mia
questo un articolo di esempio:
https://edgarluque.com/blog/rust-hashmap/

Ho smartellato qua e la per creare una versione della mappa che funziona, ma devo capire quello che ho fatto
in particolare:
- lifetime
- option get una reference mutable 


sembra che l'installazione di perf per wsl2 non fosse corretta provo installando delle lib

sudo apt install libdwarf-dev libelf-dev libnuma-dev libunwind-dev \
libnewt-dev libdwarf++0 libelf++0 libdw-dev libbfb0-dev \
systemtap-sdt-dev libssl-dev libperl-dev python-dev-is-python3 \
binutils-dev libiberty-dev libzstd-dev libcap-dev libbabeltrace-dev

e poi 
`cd WSL2-Linux-Kernel/tools/perf`
`make -j8 # parallel build`
`sudo cp perf /usr/local/bin`



Rust per i tipi iXX applica lo shit aritmetico o logico a seconda del segno:
se il numero è negativo applica lo shift >> arimetico, inserendo un 1
se il numero è positivo applica lo shift >> logico, inserendo uno 0
questo avviene sulla stessa variabile

il problema si pone se si porta del codice da java che usa >> (lo >>> è uguale a >> in java).
In quel caso la cosa da fare non è usare un tipo iXXX ma un tipo uXXX
Rimane da capire cosa succedere se viene applicato sia >> che >>> e non si sa a priori il segno dell'operazione
in quel caso servono dei cast al volo

### 09/02/2024
se voglio vedere i flamegraph devo eseguire sempre i due comandi
`echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid`
`echo 0 |sudo tee /proc/sys/kernel/kptr_restrict`
le info di flamegraph non sono utili (è tutto io) forse è un problema di wsl2

da capire meglio e sistemare la parte di conversione capendo i64 vs u64


### 07/02/2024
provando ad usare 
cargo flamegraph -- /home/gio/rust/1-billion-row-challenge/measurements.txt
non funziona più ottengo sempre 
1: No stack counts found
neanche con l'hack riportato sotto funziona


### 05/02/2024
ho trovato problemi nel porting delle routine per overflow/underflow capire meglio le differenze rispetto a Java che ha solo i signed
capire cone in java avviene la gestione dell'overflow e anche come avviene in rust
con la versione che fa il swar dei nomi e il parse del num sono sceso a 42 secondi circa

### 04/02/2024

provata installare rust e flamegraph su wsl2 wsl2 usa un kernel ad hoc ci cuole una versione specifica di perf di cui fare la build 
https://stackoverflow.com/questions/60237123/is-there-any-method-to-run-perf-under-wsl
https://gist.github.com/abel0b/b1881e41b9e1c4b16d84e5e083c38a13

sotto wsl ci mette 110/81 sec i tempi sembrano in linea (ottimo)

dopo aver installato perf funziona ma l'svg mostra unkown nei method name
trovato questo
https://users.rust-lang.org/t/flamegraph-shows-every-caller-is-unknown/52408/2
funziona

per abilitare la riga 

`line.split_once(|&c| c == b';').unwrap();`
serve la direttiva
`#![feature(slice_split_once)]`

con il passaggio a u8 il tempo scende a 63.745 ms

con il passaggio a fxmap il tempo scend a 50.893sec

il tempo per measurement_test è di 5.037 secondi

leggere meglio:
https://stackoverflow.com/questions/57340308/how-does-rusts-128-bit-integer-i128-work-on-a-64-bit-system

### 03/02/2024
Aggiornato rust con rustup update rustc (1.75 1.77.0-nightly)
parto da  https://curiouscoding.nl/posts/1brc/

Per compilare nativo ho aggiunto la dir .cargo con dentro un file di conf con la direttiva per il compilatore
https://doc.rust-lang.org/cargo/reference/config.html
Esempio di contenuto preso da qui
https://github.com/simd-lite/simd-json/blob/aa85faf/.cargo/config
cargo --release -v mostra i flag e si può verificare se è attivato target-cpu-native
la versione debug gira in 333.228ms (in questa non c'era cpu-native come target con target native ottengo 328.007ms quindi poca differenza) la release in 89.287ms

per installare flamegraph con
cargo install flamegraph
ho dovuto aggiungere le seguenti sezioni alla conf di cargo

[http]
check-revoke = false

sembra essere un problema windows

qui le info aggiuntive da approfondire https://github.com/flamegraph-rs/flamegraph
installazione di dtrace 
https://learn.microsoft.com/it-it/windows-hardware/drivers/devtest/dtrace
aggiunto dtrace al path e eseguito comando
bcdedit /set dtrace ON 




