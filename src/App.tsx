import {MutableRefObject, useEffect, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import "./App.css";
import {isRegistered, register} from "@tauri-apps/plugin-global-shortcut";
import {
    currentMonitor,
    getCurrentWindow,
    PhysicalPosition,
    PhysicalSize
} from "@tauri-apps/api/window";

interface SearchResult {
    filename: string,
    filepath: string,
    filetype: string,
    isFromCache: null | boolean
}

function App() {
    const [infoMessage, setInfoMessage] = useState("");
    const [filename, setFilename] = useState("");
    const [results, setResults] = useState<SearchResult[]>([]);
    const [focusedIndex, setFocusedIndex] = useState(-1);
    const searchResultContainerRef = useRef<HTMLDivElement | null>(null);
    const [searchInputRef, setFocus] = useFocus<HTMLInputElement | null>();
    const dNone = infoMessage.length === 0 ? 'd-none' : '';

    useEffect(() => {
        if (filename !== '') {
            searchFromCache();
        } else {
            setInfoMessage('');
            setFocusedIndex(-1);
            resizeWindowToFitContent(0, false);
        }
    }, [filename]);

    async function launch(result: SearchResult) {
        setFilename('');
        setResults([]);
        setInfoMessage('');
        setFocusedIndex(-1);

        await invoke("launch", {filepath: result.filepath});
        await hideWindow();
    }

    async function hideWindow() {
        console.log('hideWindow')
        setResults([]);
        setInfoMessage('');
        setFilename('');

        const mainWindow = getCurrentWindow();
        const searchInputSize = searchInputRef.current ? searchInputRef.current.clientHeight : 53;

        await mainWindow.setSize(new PhysicalSize({
            width: 600,
            height: searchInputSize
        }))
        await mainWindow.hide();
    }

    async function showWindow() {
        const mainWindow = getCurrentWindow();
        const windowSize = await mainWindow.innerSize();
        const monitor = await currentMonitor();

        await mainWindow.setPosition(new PhysicalPosition({
            x: monitor ? monitor.size.width / 2 - windowSize.width / 2 : 300,
            y: 100
        }))
        await mainWindow.setFocus();
        await mainWindow.show();
        await invoke("click_window");

        setResults([]);
        setInfoMessage('');
        setFilename('');
    }

    useEffect(() => {
        const registerShortCuts = async () => {
            const registered = await isRegistered("ctrl+space");
            if (!registered) {
                await register("ctrl+space", async (shortcut) => {
                    if (shortcut.state === 'Pressed') {
                        const mainWindow = getCurrentWindow();
                        const isVisible = await mainWindow.isVisible();

                        if (isVisible) {
                            hideWindow();
                        } else {
                            showWindow();
                        }
                    }
                });
            }
        };

        registerShortCuts();
        setFocus();
    }, []);

    function handleUserKeyPress(e: KeyboardEvent) {
        if (e.key === 'ArrowDown' || e.key === 'Tab' && !e.getModifierState('Alt') && !e.getModifierState('Shift')) {
            e.preventDefault();
            if (focusedIndex < results.length-1) {
                let index = focusedIndex + 1;
                setFocusedIndex(index);
                searchResultContainerRef.current?.scrollTo({ left: 0, top: (index-4)*63, behavior: "smooth" })
            }
        } else if (e.key === 'ArrowUp' || e.key === 'Tab' && !e.getModifierState('Alt') && e.getModifierState('Shift')) {
            e.preventDefault();
            if (focusedIndex > 0) {
                let index = focusedIndex - 1;
                setFocusedIndex(index);
                searchResultContainerRef.current?.scrollTo({ left: 0, top: (index-4)*63, behavior: "smooth" })
            }
        } else if (e.key === 'Enter' && focusedIndex >= 0) {
            launch(results[focusedIndex]);
        } else if (e.key === 'Escape') {
            hideWindow();
        }
    }

    useEffect(() => {
        window.addEventListener('blur', hideWindow);
        return () => {
            window.removeEventListener('blur', hideWindow);
        }
    }, [hideWindow]);

    useEffect(() => {
        window.addEventListener("keydown", handleUserKeyPress);
        return () => {
            window.removeEventListener("keydown", handleUserKeyPress);
        };
    }, [handleUserKeyPress]);

    async function resizeWindowToFitContent(nbItems: false | number = false, infoMessage: boolean) {
        nbItems = nbItems ? nbItems : results.length;
        const itemHeight = 63;
        const maxNbItems = 8;

        const searchInputSize = searchInputRef.current ? searchInputRef.current.clientHeight : 53;
        let infoMessageHeight = infoMessage ? 30 : 0;

        const container = searchResultContainerRef.current;
        if (container) {
            const searchResultContainerHeight = (Math.min(maxNbItems, nbItems) * itemHeight);
            const mainWindow = getCurrentWindow();
            const width = await mainWindow.innerSize(); // Set a fixed or desired width.

            if (searchResultContainerRef.current) {
                searchResultContainerRef.current.style.overflowY = nbItems > maxNbItems ? 'scroll' : 'hidden';
                searchResultContainerRef.current.style.height = searchResultContainerHeight + "px";
            }

            const height = searchResultContainerHeight + searchInputSize + infoMessageHeight;

            const size = {
                width: width.width,
                height: height,
            };

            // Adjust the window size
            await mainWindow.setSize(new PhysicalSize(size));
        }
    }

    async function searchFromCache() {
        setInfoMessage('Searching for "' + filename + '" in the cache...');

        await invoke("search_from_cache", {filename: filename})
            .then(response => response as string)
            .then(response => JSON.parse(response) as SearchResult[])
            .then(async (response) => {
                const cacheSortedResults = response.sort((a, b) => {
                    const aIsExe = a.filename.endsWith(".exe") ? -1 : 0;
                    const bIsExe = b.filename.endsWith(".exe") ? 0 : -1;
                    return bIsExe - aIsExe || a.filename.localeCompare(b.filename);
                });

                response.forEach(result => result.isFromCache = true);

                setResults(cacheSortedResults);
                setInfoMessage(cacheSortedResults.length + ' results for "' + filename + '" from cache.');
                await resizeWindowToFitContent(cacheSortedResults.length, true)
            })
    }

    async function searchFileSystem() {
        const startTime = Date.now();
        setInfoMessage('Searching for "' + filename + '" in the filesystem...');
        invoke("search", {filename: filename})
            .then(response => response as string)
            .then(response => JSON.parse(response) as SearchResult[])
            .then(async (response) => {
                const sortedResults = response.sort((a, b) => {
                    const aIsExe = a.filename.endsWith(".exe") ? -1 : 0;
                    const bIsExe = b.filename.endsWith(".exe") ? 0 : -1;
                    return bIsExe - aIsExe || a.filename.localeCompare(b.filename);
                });

                const endTime = Date.now();
                const duration = ((endTime - startTime)/1000).toFixed(1)
                setResults(sortedResults);
                setInfoMessage(sortedResults.length + ' results for "' + filename + '" (' + duration + 's)');
                await resizeWindowToFitContent(sortedResults.length, true);
            });
    }

    async function search() {
        setResults([]);

        setInfoMessage('Searching for "' + filename + '"...');

        const searchInputSize = searchInputRef.current ? searchInputRef.current.clientHeight : 53;
        const mainWindow = getCurrentWindow();
        await mainWindow.setSize(new PhysicalSize({
            width: 600,
            height: searchInputSize + 30
        }))
        await searchFromCache()
            .then(() => {
                searchFileSystem();
            })
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
                    ref={searchInputRef}
                    value={filename}
                    placeholder="What are you looking for?"
                    autoFocus
                    autoComplete="off"
                />
            </form>
            <div>
                {infoMessage ? <p className={"info-message " + dNone}>{infoMessage}</p> : null}
                <div ref={searchResultContainerRef}>
                    {
                        results.map((item, index) => (
                            <SearchResultComponent key={index} result={item} launch={launch} focused={index === focusedIndex}/>
                        ))
                    }
                </div>
            </div>
        </main>
    );
}

function SearchResultComponent({result, launch, focused}: { result: SearchResult, launch: (result: SearchResult) => {}, focused: boolean }) {
    function launchProgram() {
        launch(result);
    }

    let extension = result.filename.split('.').pop();
    extension = extension ? extension : "";

    let icon = result.filetype === "file" ? <>{extension}</> :
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" className="bi bi-folder"
             viewBox="0 0 16 16">
            <path
                d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a2 2 0 0 1 .342-1.31zM2.19 4a1 1 0 0 0-.996 1.09l.637 7a1 1 0 0 0 .995.91h10.348a1 1 0 0 0 .995-.91l.637-7A1 1 0 0 0 13.81 4zm4.69-1.707A1 1 0 0 0 6.172 2H2.5a1 1 0 0 0-1 .981l.006.139q.323-.119.684-.12h5.396z"/>
        </svg>;


    let fontSize = result.filetype === "file" ? 80 / extension.length : 20;
    fontSize = Math.min(fontSize, 20);

    let cacheIcon = result.isFromCache ?
        <span title="This result commes from the cache" className="result-from-cache"><svg
            xmlns="http://www.w3.org/2000/svg" width="16" height="16"
            fill="currentColor" className="bi bi-database"
            viewBox="0 0 16 16">
            <path
                d="M4.318 2.687C5.234 2.271 6.536 2 8 2s2.766.27 3.682.687C12.644 3.125 13 3.627 13 4c0 .374-.356.875-1.318 1.313C10.766 5.729 9.464 6 8 6s-2.766-.27-3.682-.687C3.356 4.875 3 4.373 3 4c0-.374.356-.875 1.318-1.313M13 5.698V7c0 .374-.356.875-1.318 1.313C10.766 8.729 9.464 9 8 9s-2.766-.27-3.682-.687C3.356 7.875 3 7.373 3 7V5.698c.271.202.58.378.904.525C4.978 6.711 6.427 7 8 7s3.022-.289 4.096-.777A5 5 0 0 0 13 5.698M14 4c0-1.007-.875-1.755-1.904-2.223C11.022 1.289 9.573 1 8 1s-3.022.289-4.096.777C2.875 2.245 2 2.993 2 4v9c0 1.007.875 1.755 1.904 2.223C4.978 15.71 6.427 16 8 16s3.022-.289 4.096-.777C13.125 14.755 14 14.007 14 13zm-1 4.698V10c0 .374-.356.875-1.318 1.313C10.766 11.729 9.464 12 8 12s-2.766-.27-3.682-.687C3.356 10.875 3 10.373 3 10V8.698c.271.202.58.378.904.525C4.978 9.71 6.427 10 8 10s3.022-.289 4.096-.777A5 5 0 0 0 13 8.698m0 3V13c0 .374-.356.875-1.318 1.313C10.766 14.729 9.464 15 8 15s-2.766-.27-3.682-.687C3.356 13.875 3 13.373 3 13v-1.302c.271.202.58.378.904.525C4.978 12.71 6.427 13 8 13s3.022-.289 4.096-.777c.324-.147.633-.323.904-.525"/>
        </svg></span> : ''

    return (
        <div onClick={launchProgram} className={"search-result " + (focused ? 'focused' : '')}>
            <div className="icon" style={{fontSize: fontSize}}>{icon}</div>
            <div className="body">
                <div className="name">{result.filename} {cacheIcon}</div>
                <div className="path">{result.filepath}</div>
            </div>
        </div>
    );
}

function useFocus<T>(defaultRef: any = null): [MutableRefObject<T>, () => void] {
    const htmlElRef = useRef(defaultRef)
    const setFocus = () => {
        if (htmlElRef.current) {
            htmlElRef.current.focus();
            htmlElRef.current.click();
        }
    }

    return [htmlElRef, setFocus]
}

export default App;
