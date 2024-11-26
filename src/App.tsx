import {MouseEventHandler, useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import "./App.css";
import {isRegistered, register} from "@tauri-apps/plugin-global-shortcut";
import {Window} from "@tauri-apps/api/window";

interface SearchResult {
    filename: string,
    filepath: string
}

function App() {
    const [infoMessage, setInfoMessage] = useState("");
    const [filename, setFilename] = useState("");
    const [results, setResults] = useState<SearchResult[]>([]);

    async function launch(result: SearchResult) {
        const mainWindow = Window.getCurrent();
        const response = await invoke("launch", {filepath: result.filepath});
        mainWindow.hide();
    }

    useEffect(() => {
        const registerShortCuts = async () => {
            const registered = await isRegistered("ctrl+space");
            if (!registered) {
                await register(["ctrl+space"], async (shortcut) => {
                    if (shortcut.state === 'Pressed') {
                        const mainWindow = Window.getCurrent();
                        const isVisible = await mainWindow.isVisible();
                        if (isVisible) {
                            await mainWindow.hide();
                        } else {
                            await mainWindow.show();
                        }
                    }
                });
            }
        };

        registerShortCuts();
    }, []);

    async function search() {
        setResults([]);

        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        setInfoMessage("Searching for " + filename);
        let response: string = await invoke("search", {filename: filename});
        let searchResults = JSON.parse(response) as SearchResult[];

        setInfoMessage("");
        setResults(searchResults);
    }

    return (
        <main className="container">
            <form
                className="row"
                onSubmit={(e) => {
                    e.preventDefault();
                    search();
                }}
            >
                <input
                    id="search-input"
                    onChange={(e) => setFilename(e.currentTarget.value)}
                    placeholder="What are you looking for?"
                />
            </form>
            <div>
                {infoMessage ? <p className="info-message">{infoMessage}</p> : null}
                {
                    results.map((item, index) => (
                        <SearchResultComponent key={index} result={item} launch={launch} />
                    ))
                }
            </div>
        </main>
    );
}

function SearchResultComponent({result, launch}: { result: SearchResult, launch: (result: SearchResult) => {} }) {
    function launchProgram() {
        launch(result);
    }

    return (
        <div onClick={launchProgram} className="search-result">
            {result.filename}
        </div>
    );
}

export default App;
