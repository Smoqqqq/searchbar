import {useEffect, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import "./App.css";
import {isRegistered, register} from "@tauri-apps/plugin-global-shortcut";
import {
    currentMonitor,
    getCurrentWindow,
    PhysicalPosition,
    PhysicalSize
} from "@tauri-apps/api/window";

interface FileSystemEntry {
    path: string,
    name: string,
    is_dir: boolean,
}

function App() {
    const [fileSystemMapped, setFileSystemMapped] = useState(false);
    const [infoMessage, setInfoMessage] = useState("");
    const [filename, setFilename] = useState("");
    const [results, setResults] = useState<FileSystemEntry[]>([]);
    const [focusedIndex, setFocusedIndex] = useState(0);
    const searchResultContainerRef = useRef<HTMLDivElement | null>(null);
    const searchInputRef = useRef(null);
    const dNone = infoMessage.length === 0 ? 'd-none' : '';
    const [page, setPage] = useState(0);
    const [searching, setSearching] = useState(false);
    const [reachedBottom, setReachedBottom] = useState(false);

    useEffect(() => {
        if (!fileSystemMapped) {
            console.error("FileSystem not mapped.")
            return;
        }

        // A ref to store the timeout ID
        const debounceTimer = setTimeout(() => {
            setReachedBottom(false);
            setPage(0);
            setFocusedIndex(0);

            if (filename !== '') {
                search(); // Trigger the search function
                resizeWindowToFitContent(0, true);
            } else {
                setResults([]);
                setInfoMessage('');
                resizeWindowToFitContent(0, false);
            }
        }, 10); // Wait 500ms after the user stops typing

        // Cleanup the timeout on each new keystroke
        return () => clearTimeout(debounceTimer);
    }, [filename]);

    async function launch(result: FileSystemEntry) {
        setFilename('');
        setResults([]);
        setInfoMessage('');
        setFocusedIndex(0);

        await invoke("launch", {filepath: result.path});
        await hideWindow();
    }

    async function hideWindow() {
        setResults([]);
        setInfoMessage('');
        setFilename('');

        const mainWindow = getCurrentWindow();
        const searchInputSize = searchInputRef.current ? searchInputRef.current.clientHeight : 53;

        await mainWindow.setSize(new PhysicalSize({
            width: 600,
            height: searchInputSize + 30
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
        setFilename('');

        mapFileSystem();
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
    }, []);

    async function mapFileSystem() {
        // If filesystem is already mapped, skip the mapping and return early.
        if (fileSystemMapped) {
            console.log('Filesystem already mapped. Skipping...');
            return;
        }

        // Set the loading message immediately when mapping starts.
        setInfoMessage('Mapping filesystem. Go grab a coffee, this will take a few minutes...');
        await resizeWindowToFitContent(0, true)

        try {
            const startTime = Date.now();
            const response = await invoke('map_filesystem');

            if (response === 'Already mapped') {
                setInfoMessage('');
                await resizeWindowToFitContent(0, false)
            } else {
                const endTime = Date.now();
                const duration = ((endTime - startTime) / 1000).toFixed(1)
                setInfoMessage('Finished mapping filesystem in ' + duration + 's');
            }

            // Mark the filesystem as mapped after the operation is complete.
            setFileSystemMapped(true);
        } catch (error) {
            console.error("Error during filesystem mapping:", error);
            setInfoMessage('Sorry, we failed to map filesystem. Please restart app and try again');
        }
    }

    function handleUserKeyPress(e: KeyboardEvent) {
        if (e.key === 'ArrowDown' || e.key === 'Tab' && !e.getModifierState('Alt') && !e.getModifierState('Shift')) {
            e.preventDefault();
            if (focusedIndex < results.length - 1) {
                let index = focusedIndex + 1;
                setFocusedIndex(index);
                searchResultContainerRef.current?.scrollTo({left: 0, top: (index - 4) * 63, behavior: "smooth"})
            }
        } else if (e.key === 'ArrowUp' || e.key === 'Tab' && !e.getModifierState('Alt') && e.getModifierState('Shift')) {
            e.preventDefault();
            if (focusedIndex > 0) {
                let index = focusedIndex - 1;
                setFocusedIndex(index);
                searchResultContainerRef.current?.scrollTo({left: 0, top: (index - 4) * 63, behavior: "smooth"})
            }
        } else if (e.key === 'Enter' && focusedIndex >= 0) {
            launch(results[focusedIndex]);
        } else if (e.key === 'Escape') {
            hideWindow();
        }
    }

    // useEffect(() => {
    //     window.addEventListener('blur', hideWindow);
    //     return () => {
    //         window.removeEventListener('blur', hideWindow);
    //     }
    // }, [hideWindow]);

    useEffect(() => {
        searchResultContainerRef.current?.addEventListener('scroll', searchMoreResults);

        return () => {
            searchResultContainerRef.current?.removeEventListener('scroll', searchMoreResults)
        }
    }, [searchMoreResults])

    useEffect(() => {
        window.addEventListener("keydown", handleUserKeyPress);
        return () => {
            window.removeEventListener("keydown", handleUserKeyPress);
        };
    }, [handleUserKeyPress]);

    const performSearch = async (currentResults: FileSystemEntry[] = [], newPage = page) => {
        const startTime = Date.now();
        setSearching(true);
        setReachedBottom(false);

        try {
            const response = await invoke("search", { filename, page: newPage });
            const responseData = JSON.parse(response as string);
            const items = responseData.results.Vec as FileSystemEntry[];
            const endTime = Date.now();
            const duration = ((endTime - startTime) / 1000).toFixed(1);

            const allResults = [...currentResults, ...items];
            setResults(allResults);
            setInfoMessage(`${responseData.count.U32} results for "${filename}" (${duration}s)`);

            setSearching(false);
            if (allResults.length >= responseData.count.U32) {
                setReachedBottom(true);
            }

            await resizeWindowToFitContent(allResults.length, true);
        } catch (error) {
            console.error("Error during search:", error);
            setInfoMessage('Failed to search for files.');
            setSearching(false);
        }
    };

    async function search() {
        if (!fileSystemMapped) {
            console.error("FS mapping in progress.");
            return;
        }

        setResults([]);
        setInfoMessage('Searching for "' + filename + '"...');
        await performSearch([], 0)
    }

    async function searchMoreResults() {
        if (searchResultContainerRef.current && !searching && !reachedBottom) {
            if (searchResultContainerRef.current.scrollTop > searchResultContainerRef.current.scrollHeight - searchResultContainerRef.current.getBoundingClientRect().height * 1.5) {
                setInfoMessage('Searching more results matching "' + filename + '"...');
                setPage(page + 1);

                await performSearch(results, page+1)
            }
        }
    }

    async function resizeWindowToFitContent(nbItems: number, infoMessage: boolean) {
        const itemHeight = 63;
        const maxNbItems = 8;

        const searchInputSize = searchInputRef.current ? searchInputRef.current.clientHeight : 53;
        let infoMessageHeight = infoMessage ? 30 : 0;

        const container = searchResultContainerRef.current;
        if (container) {
            const searchResultContainerHeight = (Math.min(maxNbItems, nbItems + (reachedBottom ? 1 : 0)) * itemHeight);
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
            console.log("resized window : " + nbItems + " items, " + (infoMessage ? 'true' : 'false') + ' infoMessage ')
        }
    }

    return (
        <main className="container">
            <form
                className="row"
                onSubmit={(e) => {
                    e.preventDefault();
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
                            <FileSystemEntryComponent key={index} result={item} launch={launch}
                                                      focused={index === focusedIndex} index={index} setFocusedIndex={setFocusedIndex} />
                        ))
                    }
                    {
                        reachedBottom ? <>
                            <div className="search-result">
                                <div className="icon" style={{fontSize: 80/5}}>Done!</div>
                                <div className="body">
                                    <div className="name">You've reached the end.</div>
                                    <div className="path">Try searching for something else !</div>
                                </div>
                            </div>
                        </> : <></>
                    }
                </div>
            </div>
        </main>
    );
}

function FileSystemEntryComponent({result, launch, focused, index, setFocusedIndex}: {
    result: FileSystemEntry,
    launch: (result: FileSystemEntry) => {},
    focused: boolean,
    index: number,
    setFocusedIndex: (index: number) => {}
}) {
    function launchProgram() {
        launch(result);
    }

    let extension = result.name.split('.').pop();
    extension = extension ? extension : "";

    let icon = !result.is_dir ? <>{extension}</> :
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" className="bi bi-folder"
             viewBox="0 0 16 16">
            <path
                d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a2 2 0 0 1 .342-1.31zM2.19 4a1 1 0 0 0-.996 1.09l.637 7a1 1 0 0 0 .995.91h10.348a1 1 0 0 0 .995-.91l.637-7A1 1 0 0 0 13.81 4zm4.69-1.707A1 1 0 0 0 6.172 2H2.5a1 1 0 0 0-1 .981l.006.139q.323-.119.684-.12h5.396z"/>
        </svg>;


    let fontSize = !result.is_dir ? 80 / extension.length : 20;
    fontSize = Math.min(fontSize, 20);

    return (
        <div onClick={launchProgram} className={"search-result " + (focused ? 'focused' : '')} onMouseEnter={() => { setFocusedIndex(index) }}>
            <div className="icon" style={{fontSize: fontSize}}>{icon}</div>
            <div className="body">
                <div className="name">{result.name}</div>
                <div className="path">{result.path}</div>
            </div>
        </div>
    );
}

export default App;
