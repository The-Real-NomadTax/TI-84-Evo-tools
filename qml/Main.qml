import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.example.tiscreen

Kirigami.ApplicationWindow {
    id: root
    width: 760
    height: 640
    minimumWidth: 420
    minimumHeight: 360
    visible: true
    title: qsTr("TI Calculator")

    CaptureBackend { id: backend }

    function openPage(page) {
        pageStack.clear()
        pageStack.push(page)
    }

    function navigate(index) {
        navigationTabs.currentIndex = index
        switch (index) {
        case 0:
            openPage(homePage)
            break
        case 1:
            openPage(screenPage)
            break
        case 2:
            openPage(filesPage)
            break
        case 3:
            openPage(sendPage)
            break
        }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        title: qsTr("TI Calculator")
        titleIcon: "accessories-calculator"
        actions: [
            Kirigami.Action {
                text: qsTr("Main Menu")
                icon.name: "go-home"
                onTriggered: root.navigate(0)
            },
            Kirigami.Action {
                text: qsTr("Screen Capture")
                icon.name: "camera-photo"
                onTriggered: root.navigate(1)
            },
            Kirigami.Action {
                text: qsTr("Device Files")
                icon.name: "folder"
                onTriggered: root.navigate(2)
            },
            Kirigami.Action {
                text: qsTr("Send Files")
                icon.name: "document-send"
                onTriggered: root.navigate(3)
            }
        ]
    }

    pageStack.initialPage: homePage

    footer: Controls.TabBar {
        id: navigationTabs
        currentIndex: 0

        Controls.TabButton {
            text: qsTr("Menu")
            icon.name: "go-home"
            onClicked: root.navigate(0)
        }

        Controls.TabButton {
            text: qsTr("Capture")
            icon.name: "camera-photo"
            onClicked: root.navigate(1)
        }

        Controls.TabButton {
            text: qsTr("Files")
            icon.name: "folder"
            onClicked: root.navigate(2)
        }

        Controls.TabButton {
            text: qsTr("Send")
            icon.name: "document-send"
            onClicked: root.navigate(3)
        }
    }

    Component {
        id: homePage

        Kirigami.Page {
            title: qsTr("Main Menu")

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - Kirigami.Units.gridUnit * 2,
                                Kirigami.Units.gridUnit * 28)
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Heading {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("TI Calculator Tools")
                    level: 1
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Kirigami.Units.gridUnit * 4
                    text: qsTr("Screen Capture")
                    icon.name: "camera-photo"
                    onClicked: root.navigate(1)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Capture the calculator display and save it as a PNG.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Kirigami.Units.gridUnit * 4
                    text: qsTr("Check Device Files")
                    icon.name: "folder"
                    enabled: !backend.busy
                    onClicked: {
                        root.navigate(2)
                        backend.refreshFiles()
                    }
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Read programs, lists, images, and other variables from the device directory.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Kirigami.Units.gridUnit * 4
                    text: qsTr("Send a File")
                    icon.name: "document-send"
                    enabled: !backend.busy
                    onClicked: root.navigate(3)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Choose a TI variable file on this computer and transfer it to the calculator.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }

    Component {
        id: screenPage

        Kirigami.Page {
            id: capturePage
            title: qsTr("Screen Capture")

            Timer {
                id: autoCaptureTimer
                interval: autoCaptureInterval.value * 10
                running: autoCaptureSwitch.checked && capturePage.visible
                repeat: true
                onTriggered: {
                    if (!backend.busy) {
                        backend.capture(cursorCheck.checked)
                    }
                }
            }

            actions: [
                Kirigami.Action {
                    text: qsTr("Capture")
                    icon.name: "camera-photo"
                    enabled: !backend.busy
                    onTriggered: backend.capture(cursorCheck.checked)
                }
            ]

            ColumnLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.largeSpacing

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Kirigami.Theme.alternateBackgroundColor
                    radius: Kirigami.Units.smallSpacing

                    Image {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.largeSpacing
                        source: backend.imageUrl
                        fillMode: Image.PreserveAspectFit
                        cache: false
                        smooth: false
                        visible: source.toString().length > 0
                    }

                    Kirigami.PlaceholderMessage {
                        anchors.centerIn: parent
                        width: parent.width - Kirigami.Units.gridUnit * 4
                        visible: backend.imageUrl.length === 0 && !backend.busy
                        text: qsTr("No capture yet")
                        explanation: qsTr("Connect a supported TI calculator over USB.")
                        icon.name: "accessories-calculator"
                    }

                    Controls.BusyIndicator {
                        anchors.centerIn: parent
                        running: backend.busy && !autoCaptureSwitch.checked
                        visible: running
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Controls.CheckBox {
                        id: cursorCheck
                        text: qsTr("Include cursor")
                        enabled: !backend.busy
                    }

                    Item { Layout.fillWidth: true }

                    Controls.Button {
                        text: qsTr("Capture Screen")
                        icon.name: "camera-photo"
                        enabled: !backend.busy
                        onClicked: backend.capture(cursorCheck.checked)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.largeSpacing

                    Controls.Switch {
                        id: autoCaptureSwitch
                        text: qsTr("Auto capture")
                    }

                    Item { Layout.fillWidth: true }

                    Controls.Label {
                        text: qsTr("Interval:")
                    }

                    Controls.SpinBox {
                        id: autoCaptureInterval
                        from: 1
                        to: 30000
                        value: 500
                        stepSize: 25
                        editable: true
                        enabled: autoCaptureSwitch.checked
                        Accessible.name: qsTr("Auto-capture interval in seconds")

                        textFromValue: function(value, locale) {
                            return Number(value / 100).toLocaleString(locale, "f", 2)
                        }

                        valueFromText: function(text, locale) {
                            return Math.round(Number.fromLocaleString(locale, text) * 100)
                        }
                    }

                    Controls.Label {
                        text: qsTr("seconds")
                    }
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: autoCaptureSwitch.checked
                    text: qsTr("Very fast capture intervals, especially below 0.25 seconds, may cause connection issues or make the calculator temporarily unresponsive.")
                    type: Kirigami.MessageType.Warning
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    type: backend.status.indexOf("failed") >= 0
                        ? Kirigami.MessageType.Error
                        : Kirigami.MessageType.Information
                }
            }
        }
    }

    Component {
        id: filesPage

        Kirigami.Page {
            id: deviceFilesPage
            title: qsTr("Device Files")
            property var deviceFiles: []

            function loadDeviceFiles() {
                try {
                    deviceFiles = JSON.parse(backend.filesJson)
                } catch (error) {
                    deviceFiles = []
                }
            }

            Connections {
                target: backend
                function onFilesJsonChanged() {
                    deviceFilesPage.loadDeviceFiles()
                }
            }

            Controls.Dialog {
                id: deleteDialog
                anchors.centerIn: parent
                modal: true
                title: qsTr("Delete device file?")
                standardButtons: Controls.Dialog.Ok | Controls.Dialog.Cancel
                property var pendingFile: null

                contentItem: Controls.Label {
                    width: Kirigami.Units.gridUnit * 22
                    text: deleteDialog.pendingFile
                        ? qsTr("Permanently delete %1 (type %2) from the calculator?")
                            .arg(deleteDialog.pendingFile.name)
                            .arg(deleteDialog.pendingFile.type)
                        : ""
                    wrapMode: Text.WordWrap
                }

                onAccepted: {
                    if (pendingFile) {
                        backend.deleteFile(pendingFile.name,
                                           pendingFile.wireName,
                                           pendingFile.type)
                    }
                    pendingFile = null
                }
                onRejected: pendingFile = null
            }

            actions: [
                Kirigami.Action {
                    text: qsTr("Refresh")
                    icon.name: "view-refresh"
                    enabled: !backend.busy
                    onTriggered: backend.refreshFiles()
                }
            ]

            ColumnLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.largeSpacing

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Kirigami.Theme.alternateBackgroundColor
                    radius: Kirigami.Units.smallSpacing

                    ListView {
                        id: deviceFileList
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        clip: true
                        spacing: Kirigami.Units.smallSpacing
                        model: deviceFilesPage.deviceFiles

                        delegate: Controls.ItemDelegate {
                            required property var modelData
                            width: ListView.view.width
                            enabled: !backend.busy

                            contentItem: RowLayout {
                                spacing: Kirigami.Units.largeSpacing

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 0

                                    Controls.Label {
                                        Layout.fillWidth: true
                                        text: modelData.name
                                        font.bold: true
                                        elide: Text.ElideRight
                                    }

                                    Controls.Label {
                                        Layout.fillWidth: true
                                        text: qsTr("%1 bytes · type %2 · version %3 · %4")
                                            .arg(modelData.size)
                                            .arg(modelData.type)
                                            .arg(modelData.version)
                                            .arg(modelData.memory ? qsTr("archived") : qsTr("RAM"))
                                        opacity: 0.75
                                        elide: Text.ElideRight
                                    }
                                }

                                Controls.Button {
                                    text: qsTr("Delete")
                                    icon.name: "edit-delete"
                                    enabled: modelData.wireName.length > 0 && !backend.busy
                                    onClicked: {
                                        deleteDialog.pendingFile = modelData
                                        deleteDialog.open()
                                    }
                                }
                            }
                        }

                        Controls.ScrollBar.vertical: Controls.ScrollBar {}
                    }

                    Kirigami.PlaceholderMessage {
                        anchors.centerIn: parent
                        width: parent.width - Kirigami.Units.gridUnit * 4
                        visible: deviceFileList.count === 0 && !backend.busy
                        text: qsTr("No device files loaded")
                        explanation: qsTr("Press Refresh to read files from the connected calculator.")
                        icon.name: "folder"
                    }
                }

                Controls.BusyIndicator {
                    Layout.alignment: Qt.AlignHCenter
                    running: backend.busy
                    visible: running
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    type: backend.status.indexOf("failed") >= 0
                        ? Kirigami.MessageType.Error
                        : Kirigami.MessageType.Information
                }
            }
        }
    }

    Component {
        id: sendPage

        Kirigami.Page {
            id: sendFilePage
            title: qsTr("Send Files")
            property string selectedPath: ""

            FileDialog {
                id: fileDialog
                title: qsTr("Choose a TI calculator file")
                fileMode: FileDialog.OpenFile
                nameFilters: [
                    qsTr("TI-84 EVO programs (*.8xp2)"),
                    qsTr("Python programs (*.py)"),
                    qsTr("All files (*)")
                ]
                onAccepted: {
                    sendFilePage.selectedPath = decodeURIComponent(
                        selectedFile.toString().replace(/^file:\/\//, ""))
                }
            }

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - Kirigami.Units.gridUnit * 2,
                                Kirigami.Units.gridUnit * 30)
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Heading {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Transfer to Calculator")
                    level: 1
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Select a native TI-84 EVO .8xp2 program or a Python .py file. Python filenames must contain 1–8 letters.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    text: qsTr("Choose File…")
                    icon.name: "document-open"
                    enabled: !backend.busy
                    onClicked: fileDialog.open()
                }

                Controls.TextField {
                    Layout.fillWidth: true
                    text: sendFilePage.selectedPath
                    readOnly: true
                    placeholderText: qsTr("No host file selected")
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Kirigami.Units.gridUnit * 3
                    text: qsTr("Send to Device")
                    icon.name: "document-send"
                    enabled: sendFilePage.selectedPath.length > 0 && !backend.busy
                    onClicked: backend.uploadFile(sendFilePage.selectedPath)
                }

                Controls.BusyIndicator {
                    Layout.alignment: Qt.AlignHCenter
                    running: backend.busy
                    visible: running
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    type: backend.status.indexOf("failed") >= 0
                        ? Kirigami.MessageType.Error
                        : Kirigami.MessageType.Information
                }
            }
        }
    }
}
