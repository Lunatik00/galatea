use anyhow::Result;
use clap::{Parser, Subcommand};
use galatea::{Conf, experimento::Opciones, servir};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "galatea",
    about = "Probador virtual con IA generativa: demo presencial"
)]
struct Cli {
    #[command(subcommand)]
    comando: Option<Comando>,
}

#[derive(Subcommand)]
enum Comando {
    /// Levanta el servidor (también es lo que pasa sin subcomando).
    Servir,
    /// Corre personas × prendas × flujos y deja tabla.csv y hoja.html en --salida.
    /// Las imágenes salen sin marca de agua: son para evaluar, no para mostrar.
    Experimento {
        #[arg(long)]
        personas: PathBuf,
        #[arg(long, default_value = "./prendas")]
        prendas: PathBuf,
        #[arg(long, value_delimiter = ',', required = true)]
        flujos: Vec<String>,
        #[arg(long)]
        salida: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    match cli.comando {
        None | Some(Comando::Servir) => servir(Conf::desde_entorno()?).await,
        Some(Comando::Experimento {
            personas,
            prendas,
            flujos,
            salida,
        }) => {
            let conf = Conf::desde_entorno()?;
            let estado = galatea::preparar(&conf).await?;
            let r = galatea::experimento::correr(
                &estado,
                Opciones {
                    personas,
                    prendas,
                    flujos,
                    salida: salida.clone(),
                },
            )
            .await?;
            println!(
                "{} filas; tabla en {} y hoja en {}",
                r.filas.len(),
                salida.join("tabla.csv").display(),
                salida.join("hoja.html").display()
            );
            Ok(())
        }
    }
}
