

export const Dashboard = () => {
    return (
        <div>
            <h1 className="text-2xl font-bold mb-4">Dashboard</h1>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-200">
                    <h3 className="text-gray-500 text-sm font-medium">System Status</h3>
                    <p className="text-2xl font-bold text-green-600 mt-2">Healthy</p>
                </div>
                {/* More stats */}
            </div>
        </div>
    );
};
