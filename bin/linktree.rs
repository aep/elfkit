extern crate elfkit;

use std::env;
use elfkit::{ Header, types, symbol, SymbolicLinker, loader};
use std::fs::File;
use std::io::Write;

fn main() {
    let mut loader: Vec<loader::State> = env::args().skip(2).map(|s| loader::State::Path{name:s}).collect();

    let rootsym = env::args().nth(1).unwrap().into_bytes();
    loader.push(loader::State::Object{
        name:     String::from("___linker_entry"),
        symbols:  vec![symbol::Symbol{
            stype: types::SymbolType::FUNC,
            size:  0,
            value: 0,
            bind:  types::SymbolBind::GLOBAL,
            vis:   types::SymbolVis::DEFAULT,
            shndx: symbol::SymbolSectionIndex::Undefined,
            name:  rootsym.to_vec(),
            _name: 0,
        }],
        header:   Header::default(),
        sections: Vec::new(),
    });

    let mut linker = SymbolicLinker::default();
    linker.link(loader).unwrap();
    println!("lookup complete: {} objects are required", linker.objects.len());
    linker.gc();
    println!("after gc : {}", linker.objects.len());


    let mut file = File::create("link.dot").unwrap();
    writeln!(&mut file, "digraph link{{").unwrap();
    writeln!(&mut file, "    node[shape=record]").unwrap();
    writeln!(&mut file, "
    Legend [pos=\"1,1\", pin=true, shape=none, margin=0, label=<
        <table border=\"2\" cellborder=\"1\" cellspacing=\"0\" cellpadding=\"4\">
            <tr>
                <td colspan=\"2\"><b>Legend</b></td>
            </tr>
            <tr>
                <td bgcolor=\"red\">Red</td>
                <td>UNDEF</td>
            </tr>
            <tr>
                <td >Regular line</td>
                <td>GLOBAL</td>
            </tr>
            <tr>
                <td>Dashed line</td>
                <td>WEAK</td>
            </tr>
            <tr>
                <td>Dotted line</td>
                <td>COMMON</td>
            </tr>
        </table>
    >];
    ").unwrap();

    linker.write_graphviz(&mut file).unwrap();

    writeln!(&mut file, "}}").unwrap();


    //for link in linker.symtab {
    //    println!("{:?}", link.sym);
    //}

}
