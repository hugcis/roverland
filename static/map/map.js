var getUrl = window.location;
var baseUrl = getUrl.protocol + "//" + getUrl.host;
var map = L.map('map').setView([39.74739, -105], 13);

var tiles = L.tileLayer('https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token=pk.eyJ1IjoibWFwYm94IiwiYSI6ImNpejY4NXVycTA2emYycXBndHRqcmZ3N3gifQ.rJcFIG214AriISLbB6B5aw', {
    maxZoom: 18,
    attribution: 'Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, ' +
        'Imagery © <a href="https://www.mapbox.com/">Mapbox</a>',
    id: 'mapbox/dark-v9',
    tileSize: 512,
    zoomOffset: -1
}).addTo(map);
let dateStart = moment().startOf('day');
let dateEnd = dateStart.clone().add(1, "days");
let overlayLayer = null;
let available_dates = null;

async function fetchAvailableDates() {
    const response = await fetch(baseUrl + "/api/available");
    const content = await response.json();
    let available_dates = content.map((strDate) => moment(strDate));
    available_dates.sort((moment1, moment2) => moment1 - moment2);
    return available_dates;
}


fetchAvailableDates().then((val) => {
    available_dates = val;
    colorCalendar(available_dates);
});


const ctx = document.getElementById('chart').getContext('2d');
const myChart = new Chart(ctx, {
    type: 'scatter',
    data: {
        labels: [],
        datasets: [{
                xAxisID: "xAxes",
                yAxisID: "y",
                label: 'Battery level',
                data: [],
                borderColor: 'rgb(243, 102, 102)',
                fill: false,
                tension: 1.,
                borderWidth: 2.,
                pointRadius: 0.,
                showLine: true,
            },
            // {
            //     type: "scatter",
            //     label: "State",
            //     xAxisID: "xAxes",
            //     data: [],
            //     barPercentage: 10.,
            // },
        ]
    },
    options: {
        interaction: {
            intersect: false,
            mode: 'point',
        },
        scales: {
            y: {
                beginAtZero: true
            },
            xAxes: {
                ticks: {
                    callback: function(label, index, ticks) {
                        return moment(label).format("HH:mm");
                    }
                }
            },
        },
        plugins: {
            tooltip: {
                callbacks: {
                    label: function(context) {
                        var label = moment(parseInt(context.label.replaceAll(",", ""))).format("HH:mm") + " " + context.formattedValue;
                        return label; + ': (' + tooltipItem.xLabel + ', ' + tooltipItem.yLabel + ')';
                    }
                }
            }
        }
    }
});

L.Control.Command = L.Control.extend({
    options: {
        position: 'topleft',
    },
    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', PreviousDate);

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Previous";
        return controlDiv;
    }
});
L.control.command = function(options) {
    return new L.Control.Command(options);
};
L.control.command({}).addTo(map);

L.Control.CommandNext = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', NextDate);

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Next";
        return controlDiv;
    }
});
L.control.commandNext = function(options) {
    return new L.Control.CommandNext(options);
};
L.control.commandNext({}).addTo(map);

L.Control.DatePicker = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', () => colorCalendar(available_dates));

        controlDiv.innerHTML = `<div id="date-range-picker" style="background-color: white;">
            <h4>Date range</h4>
            <center>
              <input type="text" name="daterange-picker" class="form-control">
            </center>
          </div>`;

        return controlDiv;
    }
});
L.control.datePicker = function(options) {
    return new L.Control.DatePicker(options);
};
L.control.datePicker({}).addTo(map);

function getDatePickerMonthAndDates(current_month) {
    let date = moment();
    let month_obj = current_month.getElementsByClassName("month");
    if (month_obj.length == 0) {
        return Error("Date picker not loaded.")
    }
    let month_year_str = month_obj[0].innerHTML.split(" ");
    date.month(month_year_str[0]).year(month_year_str[1]);
    let tbody = current_month.getElementsByClassName("table-condensed")[0].children[1];
    let days = [...tbody.getElementsByClassName("available")].filter((item) => !item.className.includes("ends"));

    let moment_days = days.map((d) => {
        let day_of_month = parseInt(d.innerText);
        let base_date = moment(date);
        base_date.date(day_of_month);
        return {
            dom_day: d,
            moment_day: base_date
        };
    });

    return moment_days.sort((d1, d2) => d1.moment_day - d2.moment_day);
}


function isSameDay(moment1, moment2) {
    return moment1.isSame(moment2, "year") && moment1.isSame(moment2, "month") && moment1.isSame(moment2, "day");
}

function colorCalendar(dates) {
    let datepicker = document.getElementsByClassName("daterangepicker")[0];
    let current_month = datepicker.getElementsByClassName("drp-calendar left")[0];
    let next_month = datepicker.getElementsByClassName("drp-calendar right")[0];
    let current_month_days = getDatePickerMonthAndDates(current_month);
    let next_month_days = getDatePickerMonthAndDates(next_month);

    // available and days_moment are two lists of moments assumed to be sorted.
    if (!(current_month_days instanceof Error || next_month_days instanceof Error)) {
        const days_moment = current_month_days.concat(next_month_days);
        var index_avail = available_dates.length - 1;
        var index_picker = days_moment.length - 1;
        while (index_avail >= 0 && index_picker >= 0) {
            if (isSameDay(days_moment[index_picker].moment_day, available_dates[index_avail])) {
                days_moment[index_picker].dom_day.style.backgroundColor = "red";
                index_picker--;
                index_avail--;
            } else if (days_moment[index_picker].moment_day.isAfter(available_dates[index_avail])) {
                index_picker--;
            } else {
                index_avail--;
            }
        }
    }
}

$(function() {
    $('input[name="daterange-picker"]').daterangepicker({
        ranges: {
            'Today': [moment(), moment().add(1, "days")],
            'Yesterday': [moment().subtract(1, 'days'), moment()],
            'Last 7 Days': [moment().subtract(6, 'days'), moment()],
            // 'Last 30 Days': [moment().subtract(29, 'days'), moment()],
            // 'This Month': [moment().startOf('month'), moment().endOf('month')],
            // 'Last Month': [moment().subtract(1, 'month').startOf('month'), moment().subtract(1, 'month').endOf('month')]
        },
        "startDate": moment(),
        "endDate": moment(),
        "opens": "left"
    }, function(start, end, label) {
        dateStart = start;
        dateEnd = end;
        console.log('New date range selected: ' + dateStart.format('YYYY-MM-DD') + ' to ' + dateEnd.format('YYYY-MM-DD') + ' (predefined range: ' + label + ')');
        updateData(dateStart, dateEnd);
    })
});


function NextDate() {
    dateStart.add(1, 'days');
    dateEnd.add(1, 'days');
    updateData(dateStart, dateEnd);
}

function PreviousDate() {
    dateStart.add(-1, 'days');
    dateEnd.add(-1, 'days');
    updateData(dateStart, dateEnd);
}

async function fetchDataJSON(dateStart, dateEnd) {
    const response = await fetch(baseUrl + "/api/query?start=" + dateStart.format() + "&end=" + dateEnd.format());
    const content = await response.json();
    return content;
}

function drawGeoJSON(json) {
    for (const device_name in json.devices) {
        let json_device = json.devices[device_name];
        json_device.sort((obj1, obj2) => moment(obj1[5]) - moment(obj2[5]));
        let times = json_device.map((obj) => moment(obj[5]));
        let timestamps = json_device.map((obj) => {
            let date = moment(obj[5]);
            return date.hour().toString() + ":" + date.minute().toString();
        });
        let time_max = Math.max(...times);
        let time_min = Math.min(...times);

        let battery_vals = json_device.map((obj) => {
            return {
                x: moment(obj[5]),
                y: obj[3],
            };
        });
        let battery_states = json_device.map((obj) => json.states[obj[4]]);
        let speeds = json_device.map((obj) => obj[7]);

        let coords = json_device.map((d) => [d[1], d[0]]);
        let max_coords = coords.reduce(function(a, b) {
            return [Math.max(a[0], b[0]), Math.max(a[1], b[1])];
        }, [-Infinity, -Infinity]);
        let min_coords = coords.reduce(function(a, b) {
            return [Math.min(a[0], b[0]), Math.min(a[1], b[1])];
        }, [Infinity, Infinity]);
        map.setView([(max_coords[0] + min_coords[0]) / 2,
            (max_coords[1] + min_coords[1]) / 2
        ]);

        overlayLayer = L.featureGroup(coords.map((coord, index) => {
            let dtime = Date.parse(times[index]);
            let percent = (dtime - time_min) / (time_max - time_min);
            let color = "rgb(" + (percent * 255) + ", " + (120 * percent) + ", " + ((1 - percent) * 255) + ")";

            let popupContent = "";
            if (times[index]) {
                popupContent += "<p><b>" + times[index].format() + "</b></p>";
            }
            if (speeds[index] > 0) {
                popupContent += "<p>" + 3.6 * speeds[index] + " km/h</p>";
            }


            return L.circleMarker([coord[0], coord[1]], {
                radius: 3,
                fillColor: color,
                weight: 0,
                opacity: 0.5,
                fillOpacity: 0.5,
                tooltip: popupContent,
            });
        })).bindPopup((circle) => circle.options.tooltip).addTo(map);
        map.fitBounds(overlayLayer.getBounds());

        myChart.data.datasets[0].data = battery_vals;
        // myChart.data.datasets[1].data = battery_states;
        myChart.data.labels = timestamps;
        myChart.update();

        // let line = L.polyline(coords).addTo(map);
        break;
    }


}

function updateData(dateStart, dateEnd) {
    if (overlayLayer) {
        map.removeLayer(overlayLayer);
    }
    fetchDataJSON(dateStart, dateEnd)
        .then(drawGeoJSON)
        .catch(err => console.error(`Error fetching data: ${err.message}`));

}

updateData(dateStart, dateEnd);